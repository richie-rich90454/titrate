//! Direct lowering of the shared TIR to WebAssembly emission.
//!
//! This is the first backend path that consumes TIR instead of re-deriving
//! semantics from the AST. It reads interned names, class field slots, vtable
//! method slots, resolved call targets and monomorphized type arguments from
//! [`TirProgram`]. Constructs TIR does not model (or that this first path
//! cannot yet emit) produce an explicit `wasm TIR:` error rather than falling
//! back to the AST lowering.

use std::collections::HashMap;

use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, FuncType,
    Function, FunctionSection, GlobalSection, GlobalType, Ieee64, ImportSection, Instruction,
    MemorySection, MemoryType, Module, TypeSection, ValType,
};

use crate::ast::{Literal, Operator, UnOp};
use crate::tir::{
    CallTarget, FieldSlot, FunctionId, NameId, NameInterner, SimdHint, TirExpr, TirProgram,
    TirStmt, TirType, VTableId,
};

use super::runtime::{self, import_index, IMPORT_COUNT};
use super::WTy;

const HEAP_START: i32 = 65_536;

/// Physical representation of a value as observed by the shape projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmValueKind {
    Int,
    Float,
    Bool,
    Char,
    Str,
    Handle,
    Void,
}

fn value_kind(ty: WTy) -> WasmValueKind {
    match ty {
        WTy::I64 => WasmValueKind::Int,
        WTy::F64 => WasmValueKind::Float,
        WTy::Bool => WasmValueKind::Bool,
        WTy::Char => WasmValueKind::Char,
        WTy::Str => WasmValueKind::Str,
        WTy::Handle => WasmValueKind::Handle,
        WTy::Void | WTy::Unknown => WasmValueKind::Void,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    Direct,
    Native,
    Virtual,
}

/// One call emitted from a resolved TIR call target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallShape {
    pub kind: CallKind,
    /// Name as recorded in the TIR call target / vtable slot.
    pub name: String,
    /// Function name (or host import name) actually emitted.
    pub target: String,
    /// Monomorphized type arguments attached to the TIR call.
    pub type_args: Vec<String>,
    /// Vtable slot for virtual dispatch.
    pub slot: Option<u16>,
}

/// One field access emitted from a resolved TIR field slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldAccessShape {
    pub field: String,
    pub slot: u16,
    pub offset: u32,
    pub value: WasmValueKind,
    pub is_set: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionShape {
    pub name: String,
    pub capability: u16,
    pub simd: &'static str,
    pub is_public: bool,
}

/// Structural view of the emitted module, derived from the same data that
/// produces the bytes. Tests compare this instead of parsing the binary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TirWasmShape {
    pub functions: Vec<FunctionShape>,
    pub exports: Vec<String>,
    pub calls: Vec<CallShape>,
    pub field_accesses: Vec<FieldAccessShape>,
    pub string_literals: Vec<String>,
}

fn val_type(ty: WTy) -> Option<ValType> {
    match ty {
        WTy::I64 => Some(ValType::I64),
        WTy::F64 => Some(ValType::F64),
        WTy::Bool | WTy::Char | WTy::Str | WTy::Handle => Some(ValType::I32),
        WTy::Void | WTy::Unknown => None,
    }
}

fn val_to_wty(vt: ValType) -> WTy {
    match vt {
        ValType::I64 => WTy::I64,
        ValType::F64 => WTy::F64,
        ValType::I32 => WTy::Str,
        _ => WTy::Handle,
    }
}

fn wty_of(ty: &TirType, names: &NameInterner) -> Result<WTy, String> {
    match ty {
        TirType::Named { name, .. } => {
            let raw = names
                .get(*name)
                .ok_or_else(|| format!("wasm TIR: invalid type name id {}", name.as_u16()))?;
            Ok(match raw {
                "void" => WTy::Void,
                "bool" => WTy::Bool,
                "char" => WTy::Char,
                "string" => WTy::Str,
                "float" | "double" | "half" | "quad" | "f64" => WTy::F64,
                "byte" | "short" | "int" | "long" | "vast" | "uvast" | "size" | "u8" | "u16"
                | "u32" | "u64" => WTy::I64,
                _ => WTy::Handle,
            })
        }
        TirType::Ref(_) | TirType::MutRef(_) | TirType::Tuple(_) => Ok(WTy::Handle),
    }
}

fn type_label(ty: &TirType, names: &NameInterner) -> Result<String, String> {
    match ty {
        TirType::Named { name, params } => {
            let raw = names
                .get(*name)
                .ok_or_else(|| format!("wasm TIR: invalid type name id {}", name.as_u16()))?;
            if params.is_empty() {
                Ok(raw.to_string())
            } else {
                let parts = params
                    .iter()
                    .map(|p| type_label(p, names))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("{}<{}>", raw, parts.join(", ")))
            }
        }
        TirType::Ref(inner) => Ok(format!("&{}", type_label(inner, names)?)),
        TirType::MutRef(inner) => Ok(format!("&mut {}", type_label(inner, names)?)),
        TirType::Tuple(items) => {
            let parts = items
                .iter()
                .map(|p| type_label(p, names))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("({})", parts.join(", ")))
        }
    }
}

fn field_getter(ty: WTy) -> &'static str {
    match ty {
        WTy::F64 => "field_get_f64",
        WTy::Bool => "field_get_bool",
        WTy::Str | WTy::Char => "field_get_str",
        WTy::Handle => "field_get_obj",
        WTy::I64 | WTy::Void | WTy::Unknown => "field_get_i64",
    }
}

fn field_setter(ty: WTy) -> &'static str {
    match ty {
        WTy::F64 => "field_set_f64",
        WTy::Bool => "field_set_bool",
        WTy::Str | WTy::Char => "field_set_str",
        WTy::Handle => "field_set_obj",
        WTy::I64 | WTy::Void | WTy::Unknown => "field_set_i64",
    }
}

fn math_host(method: &str) -> Option<&'static str> {
    Some(match method {
        "sin" => "math_sin",
        "cos" => "math_cos",
        "tan" => "math_tan",
        "asin" => "math_asin",
        "acos" => "math_acos",
        "atan" => "math_atan",
        "atan2" => "math_atan2",
        "exp" => "math_exp",
        "ln" | "log" => "math_ln",
        "log2" => "math_log2",
        "log10" => "math_log10",
        "pow" => "math_pow",
        "cbrt" => "math_cbrt",
        "hypot" => "math_hypot",
        "abs" | "fabs" => "math_abs",
        "floor" => "math_floor",
        "ceil" => "math_ceil",
        "round" => "math_round",
        "sqrt" => "math_sqrt",
        _ => return None,
    })
}

#[derive(Clone, Copy)]
enum Label {
    Block,
    LoopHead,
}

#[derive(Clone, Copy)]
struct LoopCtx {
    exit: u32,
    head: u32,
}

#[derive(Clone)]
struct FuncPlan {
    name: String,
    /// Names of the declared TIR parameters, in order.
    param_names: Vec<NameId>,
    /// Physical parameter types. Method functions carry a leading `this`.
    params: Vec<WTy>,
    ret: WTy,
    index: u32,
    is_method: bool,
    generic: bool,
}

enum NativeCall {
    /// Exact host import already present in the runtime contract.
    Host { name: String, index: u32, ret: WTy },
    Println,
    Print,
    Math { index: u32 },
    StrLen,
    StrEq,
}

impl NativeCall {
    fn target(&self) -> String {
        match self {
            Self::Host { name, .. } => name.clone(),
            Self::Println => "print".to_string(),
            Self::Print => "print_raw".to_string(),
            Self::Math { index } => format!("host:{}", index),
            Self::StrLen => "str_len".to_string(),
            Self::StrEq => "str_eq".to_string(),
        }
    }
}

struct Emitter<'a> {
    program: &'a TirProgram,
    funcs: Vec<FuncPlan>,
    global_defs: Vec<(ValType, u64)>,
    globals: HashMap<NameId, (u32, WTy)>,

    instrs: Vec<Instruction<'static>>,
    locals: Vec<(u32, ValType)>,
    local_count: u32,
    local_map: HashMap<NameId, (u32, WTy)>,
    scopes: Vec<HashMap<NameId, (u32, WTy)>>,
    labels: Vec<Label>,
    loops: Vec<LoopCtx>,
    ret: WTy,
    is_method: bool,

    literals: Vec<Vec<u8>>,
    literal_offsets: Vec<i32>,
    shape: TirWasmShape,
}

impl<'a> Emitter<'a> {
    fn new(program: &'a TirProgram) -> Result<Self, String> {
        let mut funcs = Vec::new();
        for (index, function) in program.functions.iter().enumerate() {
            let name = program
                .name(function.name)
                .ok_or_else(|| {
                    format!(
                        "wasm TIR: function {} has invalid name id {}",
                        index,
                        function.name.as_u16()
                    )
                })?
                .to_string();
            let mut param_names = Vec::new();
            let mut params = Vec::new();
            for param in &function.params {
                program.name(param.name).ok_or_else(|| {
                    format!("wasm TIR: invalid parameter name id {}", param.name.as_u16())
                })?;
                param_names.push(param.name);
                params.push(wty_of(&param.typ, &program.names)?);
            }
            let ret = function
                .return_type
                .as_ref()
                .map(|t| wty_of(t, &program.names))
                .transpose()?
                .unwrap_or(WTy::Void);
            // TIR names methods as `Class.method`; top-level functions have no
            // separator. A method's implicit receiver is physical parameter 0.
            let is_method = name.contains('.');
            if is_method {
                params.insert(0, WTy::Handle);
            }
            funcs.push(FuncPlan {
                name,
                param_names,
                params,
                ret,
                index: IMPORT_COUNT + index as u32,
                is_method,
                generic: !function.type_params.is_empty(),
            });
        }
        if !funcs.iter().any(|f| f.name == "main") {
            return Err("wasm TIR: no entry point: TIR defines no 'main' function".to_string());
        }

        let mut globals = HashMap::new();
        let mut global_defs = Vec::new();
        for (index, global) in program.globals.iter().enumerate() {
            program.name(global.name).ok_or_else(|| {
                format!("wasm TIR: global {} has invalid name id {}", index, global.name.as_u16())
            })?;
            let ty = global
                .typ
                .as_ref()
                .map(|t| wty_of(t, &program.names))
                .transpose()?
                .unwrap_or(WTy::I64);
            if ty == WTy::Void {
                continue;
            }
            let init = match global.init {
                Some(TirExpr::Literal(Literal::Int(v), _)) => v as u64,
                Some(TirExpr::Literal(Literal::Float(f), _)) => f.to_bits(),
                Some(TirExpr::Literal(Literal::Bool(b), _)) => b as u64,
                Some(TirExpr::Literal(Literal::Char(c), _)) => c as u64,
                _ => 0,
            };
            globals.insert(global.name, (global_defs.len() as u32 + 1, ty));
            global_defs.push((val_type(ty).unwrap_or(ValType::I32), init));
        }

        let shape = TirWasmShape {
            functions: program
                .functions
                .iter()
                .zip(&funcs)
                .map(|(function, plan)| FunctionShape {
                    name: plan.name.clone(),
                    capability: function.capability.bits(),
                    simd: match function.simd {
                        SimdHint::None => "none",
                        SimdHint::Allow => "allow",
                        SimdHint::Require => "require",
                    },
                    is_public: function.is_public,
                })
                .collect(),
            exports: Vec::new(),
            calls: Vec::new(),
            field_accesses: Vec::new(),
            string_literals: Vec::new(),
        };

        Ok(Emitter {
            program,
            funcs,
            global_defs,
            globals,
            instrs: Vec::new(),
            locals: Vec::new(),
            local_count: 0,
            local_map: HashMap::new(),
            scopes: Vec::new(),
            labels: Vec::new(),
            loops: Vec::new(),
            ret: WTy::Void,
            is_method: false,
            literals: Vec::new(),
            literal_offsets: Vec::new(),
            shape,
        })
    }

    /// Validate and emit every body; errors are returned before assembly.
    fn compile_checked(mut self) -> Result<(Vec<u8>, TirWasmShape), String> {
        let mut code = CodeSection::new();
        for index in 0..self.funcs.len() {
            self.emit_function(index, &mut code)?;
        }

        let mut types = TypeSection::new();
        for (_, params, result) in runtime::IMPORTS {
            types
                .ty()
                .func_type(&FuncType::new(params.iter().copied(), *result));
        }
        for plan in &self.funcs {
            types.ty().func_type(&FuncType::new(
                plan.params.iter().copied().filter_map(val_type),
                val_type(plan.ret),
            ));
        }

        let mut imports = ImportSection::new();
        for (index, (name, _, _)) in runtime::IMPORTS.iter().enumerate() {
            imports.import("env", name, EntityType::Function(index as u32));
        }

        let mut funcsec = FunctionSection::new();
        for index in 0..self.funcs.len() {
            funcsec.function(IMPORT_COUNT + index as u32);
        }

        let mut memsec = MemorySection::new();
        memsec.memory(MemoryType {
            minimum: 256,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        let mut globalsec = GlobalSection::new();
        globalsec.global(
            GlobalType { val_type: ValType::I32, mutable: true, shared: false },
            &ConstExpr::i32_const(HEAP_START),
        );
        for (vt, init) in &self.global_defs {
            let expr = match vt {
                ValType::F64 => ConstExpr::f64_const(Ieee64::new(*init)),
                ValType::I64 => ConstExpr::i64_const(*init as i64),
                _ => ConstExpr::i32_const(*init as i32),
            };
            globalsec.global(
                GlobalType { val_type: *vt, mutable: true, shared: false },
                &expr,
            );
        }

        let mut exports = ExportSection::new();
        exports.export("memory", ExportKind::Memory, 0);
        exports.export("heap_ptr", ExportKind::Global, 0);
        self.shape.exports.push("memory".to_string());
        self.shape.exports.push("heap_ptr".to_string());
        for plan in &self.funcs {
            exports.export(&plan.name, ExportKind::Func, plan.index);
            self.shape.exports.push(plan.name.clone());
        }

        let mut data = DataSection::new();
        for (bytes, offset) in self.literals.iter().zip(self.literal_offsets.iter()) {
            let mut payload = Vec::with_capacity(4 + bytes.len());
            payload.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
            payload.extend_from_slice(bytes);
            data.active(0, &ConstExpr::i32_const(*offset), payload);
        }

        let mut module = Module::new();
        module.section(&types);
        module.section(&imports);
        module.section(&funcsec);
        module.section(&memsec);
        module.section(&globalsec);
        module.section(&exports);
        module.section(&code);
        if !self.literals.is_empty() {
            module.section(&data);
        }
        Ok((module.finish(), self.shape))
    }

    // ---- per-function state -------------------------------------------------

    fn begin_function(&mut self, index: usize) {
        let plan = self.funcs[index].clone();
        self.instrs.clear();
        self.locals.clear();
        self.local_map.clear();
        self.scopes.clear();
        self.labels.clear();
        self.loops.clear();
        self.ret = plan.ret;
        self.is_method = plan.is_method;
        self.local_count = plan.params.len() as u32;
        let offset = usize::from(plan.is_method);
        for (i, name) in plan.param_names.iter().enumerate() {
            self.local_map
                .insert(*name, ((i + offset) as u32, plan.params[i + offset]));
        }
    }

    fn emit_function(&mut self, index: usize, code: &mut CodeSection) -> Result<(), String> {
        let program = self.program;
        self.begin_function(index);
        self.emit_block(&program.functions[index].body)?;
        if self.ret != WTy::Void {
            self.zero(self.ret);
        }
        self.push(Instruction::End);
        let mut function = Function::new(self.locals.iter().copied());
        for instruction in &self.instrs {
            function.instruction(instruction);
        }
        code.function(&function);
        Ok(())
    }

    // ---- instruction helpers ------------------------------------------------

    fn push(&mut self, instruction: Instruction<'static>) {
        self.instrs.push(instruction);
    }

    fn i32c(&mut self, value: i32) {
        self.push(Instruction::I32Const(value));
    }

    fn i64c(&mut self, value: i64) {
        self.push(Instruction::I64Const(value));
    }

    fn f64c(&mut self, value: f64) {
        self.push(Instruction::F64Const(Ieee64::new(value.to_bits())));
    }

    fn call_host(&mut self, name: &str) -> Result<u32, String> {
        let index = import_index(name)
            .ok_or_else(|| format!("wasm TIR: unknown host import '{}'", name))?;
        self.push(Instruction::Call(index));
        Ok(index)
    }

    fn zero(&mut self, ty: WTy) {
        match ty {
            WTy::I64 => self.i64c(0),
            WTy::F64 => self.f64c(0.0),
            WTy::Bool | WTy::Char | WTy::Str | WTy::Handle => self.i32c(0),
            WTy::Void | WTy::Unknown => {}
        }
    }

    fn coerce(&mut self, from: WTy, to: WTy) -> Result<(), String> {
        use Instruction::*;
        if from == to || to == WTy::Unknown {
            return Ok(());
        }
        match (from, to) {
            (WTy::I64, WTy::F64) => self.push(F64ConvertI64S),
            (WTy::F64, WTy::I64) => self.push(I64TruncSatF64S),
            (WTy::Bool | WTy::Char, WTy::I64) => self.push(I64ExtendI32U),
            (WTy::I64, WTy::Bool) => {
                self.i64c(0);
                self.push(I64Ne);
            }
            (WTy::I64, WTy::Char) => self.push(I32WrapI64),
            (WTy::F64, WTy::Bool) => {
                self.f64c(0.0);
                self.push(F64Ne);
            }
            (WTy::Bool | WTy::Char, WTy::F64) => self.push(F64ConvertI32U),
            _ => return Err(format!("wasm TIR: cannot convert {:?} to {:?}", from, to)),
        }
        Ok(())
    }

    fn coerce_to_str(&mut self, from: WTy) -> Result<(), String> {
        match from {
            WTy::Str => Ok(()),
            WTy::I64 => self.call_host("to_string_i64").map(|_| ()),
            WTy::F64 => self.call_host("to_string_f64").map(|_| ()),
            WTy::Bool => self.call_host("to_string_bool").map(|_| ()),
            WTy::Char => self.call_host("to_string_char").map(|_| ()),
            WTy::Handle => self.call_host("dyn_to_string").map(|_| ()),
            WTy::Void | WTy::Unknown => {
                Err("wasm TIR: cannot stringify a valueless expression".to_string())
            }
        }
    }

    fn add_local(&mut self, ty: WTy) -> u32 {
        let index = self.local_count;
        self.locals.push((1, val_type(ty).unwrap_or(ValType::I32)));
        self.local_count += 1;
        index
    }

    fn push_scope(&mut self) {
        self.scopes.push(self.local_map.clone());
    }

    fn pop_scope(&mut self) {
        if let Some(saved) = self.scopes.pop() {
            self.local_map = saved;
        }
    }

    fn name(&self, id: NameId) -> Result<&str, String> {
        self.program
            .name(id)
            .ok_or_else(|| format!("wasm TIR: invalid name id {}", id.as_u16()))
    }

    fn func_plan(&self, id: FunctionId) -> Result<&FuncPlan, String> {
        self.funcs
            .get(id.as_u16() as usize)
            .ok_or_else(|| format!("wasm TIR: invalid function id {}", id.as_u16()))
    }

    fn lookup(&self, id: NameId) -> Result<(u32, WTy), String> {
        if let Some(found) = self.local_map.get(&id) {
            return Ok(*found);
        }
        if let Some(found) = self.globals.get(&id) {
            return Ok(*found);
        }
        Err(format!(
            "wasm TIR: unknown variable '{}'",
            self.name(id).unwrap_or("<invalid name>")
        ))
    }

    fn label_depth(&self, label: u32) -> u32 {
        self.labels.len() as u32 - 1 - label
    }

    fn br_to(&mut self, label: u32) {
        let depth = self.label_depth(label);
        self.push(Instruction::Br(depth));
    }

    fn intern_string(&mut self, value: &str) -> i32 {
        if let Some(pos) = self.literals.iter().position(|b| b.as_slice() == value.as_bytes()) {
            return self.literal_offsets[pos];
        }
        let offset = 16 + self.literals.iter().map(|b| 4 + b.len()).sum::<usize>() as i32;
        self.literals.push(value.as_bytes().to_vec());
        self.literal_offsets.push(offset);
        self.shape.string_literals.push(value.to_string());
        offset
    }

    fn record_call(
        &mut self,
        kind: CallKind,
        name: String,
        target: String,
        type_args: Vec<String>,
        slot: Option<u16>,
    ) {
        self.shape.calls.push(CallShape { kind, name, target, type_args, slot });
    }

    fn call_type_args(&self, type_args: &[TirType]) -> Result<Vec<String>, String> {
        type_args
            .iter()
            .map(|t| type_label(t, &self.program.names))
            .collect()
    }

    // ---- field resolution ---------------------------------------------------

    fn field_plan(&self, field: NameId, slot: Option<u16>) -> Result<(FieldSlot, WTy), String> {
        let name = self.name(field)?.to_string();
        let mut matches = Vec::new();
        for class in &self.program.classes {
            for candidate in &class.fields {
                if candidate.name == field && slot.is_none_or(|s| s == candidate.slot) {
                    matches.push(candidate.clone());
                }
            }
        }
        let resolved = match matches.len() {
            0 => {
                return Err(format!(
                    "wasm TIR: no class field slot for '{}'{}",
                    name,
                    slot.map(|s| format!(" (slot {})", s)).unwrap_or_default()
                ))
            }
            1 => matches.remove(0),
            _ => {
                return Err(format!(
                    "wasm TIR: field '{}' is ambiguous across class layouts; TIR needs a typed receiver",
                    name
                ))
            }
        };
        let ty = wty_of(&resolved.typ, &self.program.names)?;
        Ok((resolved, ty))
    }

    // ---- native call planning ------------------------------------------------

    fn native_plan(&self, name: &str) -> Result<NativeCall, String> {
        if let Some(index) = import_index(name) {
            let ret = runtime::IMPORTS
                .iter()
                .find(|(import, _, _)| *import == name)
                .and_then(|(_, _, result)| *result)
                .map(val_to_wty)
                .unwrap_or(WTy::Void);
            return Ok(NativeCall::Host { name: name.to_string(), index, ret });
        }
        let (module, method) = name
            .split_once('.')
            .ok_or_else(|| format!("wasm TIR: native call '{}' is not a module.method name", name))?;
        match (module, method) {
            ("io" | "console", "println" | "eprintln" | "log") => Ok(NativeCall::Println),
            ("io" | "console", "print") => Ok(NativeCall::Print),
            ("Math" | "MathAdvanced" | "MathTrig", method) => {
                let host = math_host(method).ok_or_else(|| {
                    format!("wasm TIR: math function '{}' has no host binding", method)
                })?;
                let index = import_index(host)
                    .ok_or_else(|| format!("wasm TIR: missing host import '{}'", host))?;
                Ok(NativeCall::Math { index })
            }
            ("String", "length" | "size" | "count") => Ok(NativeCall::StrLen),
            ("String", "equals") => Ok(NativeCall::StrEq),
            _ => Err(format!(
                "wasm TIR: native '{}' has no host binding in the TIR lowering",
                name
            )),
        }
    }

    fn native_ret(&self, name: &str) -> Result<WTy, String> {
        Ok(match self.native_plan(name)? {
            NativeCall::Host { ret, .. } => ret,
            NativeCall::Println | NativeCall::Print => WTy::Void,
            NativeCall::Math { .. } => WTy::F64,
            NativeCall::StrLen => WTy::I64,
            NativeCall::StrEq => WTy::Bool,
        })
    }

    // ---- statements ---------------------------------------------------------

    fn emit_block(&mut self, block: &[TirStmt]) -> Result<(), String> {
        for statement in block {
            self.emit_stmt(statement)?;
        }
        Ok(())
    }

    fn emit_stmt(&mut self, statement: &TirStmt) -> Result<(), String> {
        use Instruction::*;
        match statement {
            TirStmt::Block(body) => {
                self.push_scope();
                self.emit_block(body)?;
                self.pop_scope();
            }
            TirStmt::Expr(expression) => {
                let ty = self.emit_expr(expression)?;
                if ty != WTy::Void && ty != WTy::Unknown {
                    self.push(Drop);
                }
            }
            TirStmt::Let { name, typ, init, .. } => {
                let declared = typ
                    .as_ref()
                    .map(|t| wty_of(t, &self.program.names))
                    .transpose()?;
                let ty = match (&declared, init) {
                    (Some(ty), _) => *ty,
                    (None, Some(expression)) => self.infer_expr(expression)?,
                    (None, None) => WTy::I64,
                };
                match init {
                    Some(expression) => {
                        let from = self.emit_expr(expression)?;
                        self.coerce(from, ty)?;
                    }
                    None => self.zero(ty),
                }
                let index = self.add_local(ty);
                self.local_map.insert(*name, (index, ty));
                self.push(LocalSet(index));
            }
            TirStmt::Return(value) => {
                let ret = self.ret;
                if let Some(expression) = value {
                    let from = self.emit_expr(expression)?;
                    if ret == WTy::Void || ret == WTy::Unknown {
                        if from != WTy::Void {
                            self.push(Drop);
                        }
                    } else {
                        self.coerce(from, ret)?;
                    }
                }
                self.push(Return);
            }
            TirStmt::Break => {
                let ctx = *self.loops.last().ok_or("wasm TIR: break outside loop")?;
                self.br_to(ctx.exit);
            }
            TirStmt::Continue => {
                let ctx = *self.loops.last().ok_or("wasm TIR: continue outside loop")?;
                self.br_to(ctx.head);
            }
            TirStmt::If { cond, then_branch, else_branch, .. } => {
                let cond_ty = self.emit_expr(cond)?;
                self.coerce(cond_ty, WTy::Bool)?;
                self.labels.push(Label::Block);
                self.push(If(BlockType::Empty));
                self.push_scope();
                self.emit_block(then_branch)?;
                self.pop_scope();
                if let Some(body) = else_branch {
                    self.push(Else);
                    self.push_scope();
                    self.emit_block(body)?;
                    self.pop_scope();
                }
                self.push(End);
                self.labels.pop();
            }
            TirStmt::While { cond, body, .. } => {
                let exit = self.labels.len() as u32;
                self.labels.push(Label::Block);
                self.push(Block(BlockType::Empty));
                let head = self.labels.len() as u32;
                self.labels.push(Label::LoopHead);
                self.push(Loop(BlockType::Empty));
                self.loops.push(LoopCtx { exit, head });

                let cond_ty = self.emit_expr(cond)?;
                self.coerce(cond_ty, WTy::Bool)?;
                self.push(I32Eqz);
                let depth = self.label_depth(exit);
                self.push(BrIf(depth));

                self.push_scope();
                self.emit_block(body)?;
                self.pop_scope();
                self.br_to(head);
                self.push(End);
                self.push(End);
                self.labels.pop();
                self.labels.pop();
                self.loops.pop();
            }
            TirStmt::For { .. } => {
                return Err(
                    "wasm TIR: for-in loops are not modeled by the WASM TIR lowering yet".to_string(),
                )
            }
        }
        Ok(())
    }

    // ---- expression type inference ------------------------------------------

    fn infer_expr(&self, expression: &TirExpr) -> Result<WTy, String> {
        match expression {
            TirExpr::Literal(literal, _) => Ok(match literal {
                Literal::Int(_) => WTy::I64,
                Literal::Float(_) => WTy::F64,
                Literal::Bool(_) => WTy::Bool,
                Literal::Char(_) => WTy::Char,
                Literal::String(_) => WTy::Str,
                Literal::Null => WTy::Handle,
            }),
            TirExpr::Var(name, _) => self.lookup(*name).map(|(_, ty)| ty),
            TirExpr::This(_) => Ok(WTy::Handle),
            TirExpr::Unit(_) => Ok(WTy::Void),
            TirExpr::Unary { op, operand, .. } => match op {
                UnOp::Not => Ok(WTy::Bool),
                UnOp::Neg | UnOp::BitNot => self.infer_expr(operand),
            },
            TirExpr::Binary { op, left, right, .. } => {
                let lt = self.infer_expr(left)?;
                let rt = self.infer_expr(right)?;
                Ok(match op {
                    Operator::Eq
                    | Operator::Ne
                    | Operator::Lt
                    | Operator::Gt
                    | Operator::Le
                    | Operator::Ge
                    | Operator::And
                    | Operator::Or => WTy::Bool,
                    Operator::Add if lt == WTy::Str || rt == WTy::Str => WTy::Str,
                    _ if lt == WTy::F64 || rt == WTy::F64 => WTy::F64,
                    _ => WTy::I64,
                })
            }
            TirExpr::Assign { target, .. } => self.infer_expr(target),
            TirExpr::Cast { typ, .. } => wty_of(typ, &self.program.names),
            TirExpr::Is { .. } => Ok(WTy::Bool),
            TirExpr::Ref { value, .. } => self.infer_expr(value),
            TirExpr::Call { target, type_args: _, args, .. } => self.infer_call(target, args),
            TirExpr::Field { field, slot, .. } => Ok(self.field_plan(*field, *slot)?.1),
            TirExpr::New { .. } | TirExpr::Tuple { .. } => Ok(WTy::Handle),
            TirExpr::Ternary { then_expr, .. } => self.infer_expr(then_expr),
        }
    }

    fn infer_call(&self, target: &CallTarget, _args: &[TirExpr]) -> Result<WTy, String> {
        match target {
            CallTarget::Direct(id) => Ok(self.func_plan(*id)?.ret),
            CallTarget::Virtual { vtable, slot } => {
                let (_, target) = self.resolve_virtual(*vtable, *slot)?;
                Ok(self.func_plan(target)?.ret)
            }
            CallTarget::Native { name } => self.native_ret(self.name(*name)?),
            CallTarget::External { name } => Err(format!(
                "wasm TIR: unresolved external call target '{}'",
                self.name(*name).unwrap_or("<invalid name>")
            )),
        }
    }

    // ---- expressions --------------------------------------------------------

    fn emit_expr(&mut self, expression: &TirExpr) -> Result<WTy, String> {
        use Instruction::*;
        match expression {
            TirExpr::Literal(literal, _) => match literal {
                Literal::Int(value) => {
                    self.i64c(*value);
                    Ok(WTy::I64)
                }
                Literal::Float(value) => {
                    self.f64c(*value);
                    Ok(WTy::F64)
                }
                Literal::Bool(value) => {
                    self.i32c(i32::from(*value));
                    Ok(WTy::Bool)
                }
                Literal::Char(value) => {
                    self.i32c(*value as i32);
                    Ok(WTy::Char)
                }
                Literal::String(value) => {
                    let offset = self.intern_string(value);
                    self.i32c(offset);
                    Ok(WTy::Str)
                }
                Literal::Null => {
                    self.call_host("null_new")?;
                    Ok(WTy::Handle)
                }
            },
            TirExpr::Var(name, _) => {
                let (index, ty) = self.lookup(*name)?;
                self.push(LocalGet(index));
                Ok(ty)
            }
            TirExpr::This(_) => {
                if !self.is_method {
                    return Err("wasm TIR: 'this' used outside a method".to_string());
                }
                self.push(LocalGet(0));
                Ok(WTy::Handle)
            }
            TirExpr::Unit(_) => Ok(WTy::Void),
            TirExpr::Unary { op, operand, .. } => {
                let ty = self.emit_expr(operand)?;
                match op {
                    UnOp::Neg => {
                        if ty == WTy::F64 {
                            self.push(F64Neg);
                        } else {
                            self.coerce(ty, WTy::I64)?;
                            let tmp = self.add_local(WTy::I64);
                            self.push(LocalSet(tmp));
                            self.i64c(0);
                            self.push(LocalGet(tmp));
                            self.push(I64Sub);
                        }
                        Ok(if ty == WTy::F64 { WTy::F64 } else { WTy::I64 })
                    }
                    UnOp::BitNot => {
                        self.coerce(ty, WTy::I64)?;
                        self.i64c(-1);
                        self.push(I64Xor);
                        Ok(WTy::I64)
                    }
                    UnOp::Not => {
                        match ty {
                            WTy::F64 => {
                                self.f64c(0.0);
                                self.push(F64Ne);
                            }
                            WTy::I64 => {
                                self.i64c(0);
                                self.push(I64Ne);
                            }
                            WTy::Bool => {}
                            _ => self.coerce(ty, WTy::Bool)?,
                        }
                        self.push(I32Eqz);
                        Ok(WTy::Bool)
                    }
                }
            }
            TirExpr::Binary { op, left, right, .. } => self.emit_binary(op, left, right),
            TirExpr::Assign { target, value, .. } => match target.as_ref() {
                TirExpr::Var(name, _) => {
                    let (index, ty) = self.lookup(*name)?;
                    let from = self.emit_expr(value)?;
                    self.coerce(from, ty)?;
                    if self.local_map.contains_key(name) {
                        self.push(LocalTee(index));
                    } else {
                        self.push(GlobalSet(index));
                        self.push(GlobalGet(index));
                    }
                    Ok(ty)
                }
                TirExpr::Field { object, field, slot, .. } => {
                    self.emit_field(object, *field, *slot, Some(value))
                }
                other => Err(format!("wasm TIR: unsupported assignment target {:?}", other)),
            },
            TirExpr::Cast { value, typ, .. } => {
                let from = self.emit_expr(value)?;
                let to = wty_of(typ, &self.program.names)?;
                self.coerce(from, to)?;
                Ok(to)
            }
            TirExpr::Is { value, typ, .. } => self.emit_is(value, typ),
            TirExpr::Ref { value, .. } => {
                // shortcut: WASM erases borrows to their pointee, as the AST backend does.
                self.emit_expr(value)
            }
            TirExpr::Call { target, type_args, args, .. } => {
                self.emit_call(target, type_args, args)
            }
            TirExpr::Field { object, field, slot, .. } => {
                self.emit_field(object, *field, *slot, None)
            }
            TirExpr::New { class, args, .. } => self.emit_new(*class, args),
            TirExpr::Tuple { items, .. } => {
                self.call_host("tuple_new")?;
                let handle = self.add_local(WTy::Handle);
                self.push(LocalSet(handle));
                for (index, item) in items.iter().enumerate() {
                    self.push(LocalGet(handle));
                    self.i64c(index as i64);
                    self.emit_boxed(item)?;
                    self.call_host("tuple_set")?;
                }
                self.push(LocalGet(handle));
                Ok(WTy::Handle)
            }
            TirExpr::Ternary { condition, then_expr, else_expr, .. } => {
                let cond_ty = self.emit_expr(condition)?;
                self.coerce(cond_ty, WTy::Bool)?;
                let then_ty = self.infer_expr(then_expr)?;
                let else_ty = self.infer_expr(else_expr)?;
                let result = if then_ty != WTy::Void && then_ty != WTy::Unknown {
                    then_ty
                } else {
                    else_ty
                };
                if result == WTy::Unknown {
                    return Err("wasm TIR: cannot infer ternary result type".to_string());
                }
                let block_type = val_type(result)
                    .map(BlockType::Result)
                    .unwrap_or(BlockType::Empty);
                self.labels.push(Label::Block);
                self.push(If(block_type));
                let then_actual = self.emit_expr(then_expr)?;
                if result != WTy::Void {
                    self.coerce(then_actual, result)?;
                }
                self.push(Else);
                let else_actual = self.emit_expr(else_expr)?;
                if result != WTy::Void {
                    self.coerce(else_actual, result)?;
                }
                self.push(End);
                self.labels.pop();
                Ok(result)
            }
        }
    }

    fn emit_binary(&mut self, op: &Operator, left: &TirExpr, right: &TirExpr) -> Result<WTy, String> {
        use Instruction::*;
        let lt = self.infer_expr(left)?;
        let rt = self.infer_expr(right)?;

        if *op == Operator::Add && (lt == WTy::Str || rt == WTy::Str) {
            let a = self.emit_expr(left)?;
            self.coerce_to_str(a)?;
            let b = self.emit_expr(right)?;
            self.coerce_to_str(b)?;
            self.call_host("concat")?;
            return Ok(WTy::Str);
        }

        match op {
            Operator::Eq | Operator::Ne | Operator::Lt | Operator::Gt | Operator::Le
            | Operator::Ge => {
                if lt == WTy::Str && rt == WTy::Str {
                    self.emit_expr(left)?;
                    self.emit_expr(right)?;
                    let host = match op {
                        Operator::Eq | Operator::Ne => "str_eq",
                        Operator::Lt => "str_lt",
                        Operator::Gt => "str_gt",
                        Operator::Le => "str_le",
                        Operator::Ge => "str_ge",
                        _ => unreachable!(),
                    };
                    self.call_host(host)?;
                    if *op == Operator::Ne {
                        self.push(I32Eqz);
                    }
                    return Ok(WTy::Bool);
                }
                if lt == WTy::F64 || rt == WTy::F64 {
                    let a = self.emit_expr(left)?;
                    self.coerce(a, WTy::F64)?;
                    let b = self.emit_expr(right)?;
                    self.coerce(b, WTy::F64)?;
                    self.push(match op {
                        Operator::Eq => F64Eq,
                        Operator::Ne => F64Ne,
                        Operator::Lt => F64Lt,
                        Operator::Gt => F64Gt,
                        Operator::Le => F64Le,
                        Operator::Ge => F64Ge,
                        _ => unreachable!(),
                    });
                } else {
                    let a = self.emit_expr(left)?;
                    self.coerce(a, WTy::I64)?;
                    let b = self.emit_expr(right)?;
                    self.coerce(b, WTy::I64)?;
                    self.push(match op {
                        Operator::Eq => I64Eq,
                        Operator::Ne => I64Ne,
                        Operator::Lt => I64LtS,
                        Operator::Gt => I64GtS,
                        Operator::Le => I64LeS,
                        Operator::Ge => I64GeS,
                        _ => unreachable!(),
                    });
                }
                Ok(WTy::Bool)
            }
            Operator::And | Operator::Or => {
                let a = self.emit_expr(left)?;
                self.coerce(a, WTy::Bool)?;
                self.labels.push(Label::Block);
                self.push(If(BlockType::Result(ValType::I32)));
                if *op == Operator::And {
                    let b = self.emit_expr(right)?;
                    self.coerce(b, WTy::Bool)?;
                    self.push(Else);
                    self.i32c(0);
                } else {
                    self.i32c(1);
                    self.push(Else);
                    let b = self.emit_expr(right)?;
                    self.coerce(b, WTy::Bool)?;
                }
                self.push(End);
                self.labels.pop();
                Ok(WTy::Bool)
            }
            _ => {
                if lt == WTy::F64 || rt == WTy::F64 {
                    let a = self.emit_expr(left)?;
                    self.coerce(a, WTy::F64)?;
                    let b = self.emit_expr(right)?;
                    self.coerce(b, WTy::F64)?;
                    match op {
                        Operator::Add => self.push(F64Add),
                        Operator::Sub => self.push(F64Sub),
                        Operator::Mul => self.push(F64Mul),
                        Operator::Div => self.push(F64Div),
                        Operator::Mod => {
                            self.call_host("fmod")?;
                        }
                        _ => return Err(format!("wasm TIR: unsupported float operator {:?}", op)),
                    }
                    Ok(WTy::F64)
                } else {
                    let a = self.emit_expr(left)?;
                    self.coerce(a, WTy::I64)?;
                    let b = self.emit_expr(right)?;
                    self.coerce(b, WTy::I64)?;
                    self.push(match op {
                        Operator::Add => I64Add,
                        Operator::Sub => I64Sub,
                        Operator::Mul => I64Mul,
                        Operator::Div => I64DivS,
                        Operator::Mod => I64RemS,
                        Operator::BitAnd => I64And,
                        Operator::BitOr => I64Or,
                        Operator::BitXor => I64Xor,
                        Operator::BitShl => I64Shl,
                        Operator::BitShr => I64ShrS,
                        Operator::BitUshr => I64ShrU,
                        _ => return Err(format!("wasm TIR: unsupported integer operator {:?}", op)),
                    });
                    Ok(WTy::I64)
                }
            }
        }
    }

    fn emit_is(&mut self, value: &TirExpr, typ: &TirType) -> Result<WTy, String> {
        use Instruction::*;
        let from = self.emit_expr(value)?;
        if let TirType::Named { name, .. } = typ {
            let class = self.name(*name)?.to_string();
            if from != WTy::Handle {
                self.push(Drop);
                self.i32c(0);
                return Ok(WTy::Bool);
            }
            self.call_host("obj_class")?;
            let offset = self.intern_string(&class);
            self.i32c(offset);
            self.call_host("str_eq")?;
            return Ok(WTy::Bool);
        }
        if from != WTy::Void {
            self.push(Drop);
        }
        self.i32c(0);
        Ok(WTy::Bool)
    }

    fn emit_boxed(&mut self, expression: &TirExpr) -> Result<(), String> {
        let ty = self.emit_expr(expression)?;
        let local = self.add_local(ty);
        self.push(Instruction::LocalSet(local));
        let tag = match ty {
            WTy::F64 => runtime::TAG_F64,
            WTy::Bool => runtime::TAG_BOOL,
            WTy::Str | WTy::Char => runtime::TAG_STR,
            WTy::Handle => runtime::TAG_OBJ,
            WTy::I64 | WTy::Void | WTy::Unknown => runtime::TAG_I64,
        };
        self.i32c(tag);
        match ty {
            WTy::F64 => {
                self.i64c(0);
                self.push(Instruction::LocalGet(local));
                self.i32c(0);
            }
            WTy::I64 => {
                self.push(Instruction::LocalGet(local));
                self.f64c(0.0);
                self.i32c(0);
            }
            _ => {
                self.i64c(0);
                self.f64c(0.0);
                self.push(Instruction::LocalGet(local));
            }
        }
        Ok(())
    }

    fn resolve_virtual(&self, vtable: VTableId, slot: u16) -> Result<(String, FunctionId), String> {
        let table = self
            .program
            .vtables
            .get(vtable.as_u16() as usize)
            .ok_or_else(|| format!("wasm TIR: invalid vtable id {}", vtable.as_u16()))?;
        let entry = table
            .entries
            .get(slot as usize)
            .ok_or_else(|| format!("wasm TIR: vtable {} has no slot {}", vtable.as_u16(), slot))?;
        Ok((self.name(entry.method)?.to_string(), entry.target))
    }

    fn compile_args(&mut self, args: &[TirExpr], params: &[WTy]) -> Result<(), String> {
        if args.len() != params.len() {
            return Err(format!(
                "wasm TIR: argument count mismatch: expected {}, got {}",
                params.len(),
                args.len()
            ));
        }
        for (arg, param) in args.iter().zip(params) {
            let from = self.emit_expr(arg)?;
            self.coerce(from, *param)?;
        }
        Ok(())
    }

    fn emit_call(
        &mut self,
        target: &CallTarget,
        type_args: &[TirType],
        args: &[TirExpr],
    ) -> Result<WTy, String> {
        match target {
            CallTarget::Direct(id) => {
                let plan = self.func_plan(*id)?.clone();
                if plan.generic && type_args.is_empty() {
                    return Err(format!(
                        "wasm TIR: call to generic function '{}' has no type arguments; TIR must be monomorphized",
                        plan.name
                    ));
                }
                let labels = self.call_type_args(type_args)?;
                self.compile_args(args, &plan.params)?;
                self.push(Instruction::Call(plan.index));
                self.record_call(
                    CallKind::Direct,
                    plan.name.clone(),
                    plan.name,
                    labels,
                    None,
                );
                Ok(plan.ret)
            }
            CallTarget::Virtual { vtable, slot } => {
                let (method, target_id) = self.resolve_virtual(*vtable, *slot)?;
                let plan = self.func_plan(target_id)?.clone();
                let labels = self.call_type_args(type_args)?;
                self.compile_args(args, &plan.params)?;
                self.push(Instruction::Call(plan.index));
                self.record_call(
                    CallKind::Virtual,
                    method,
                    plan.name,
                    labels,
                    Some(*slot),
                );
                Ok(plan.ret)
            }
            CallTarget::Native { name } => {
                let name = self.name(*name)?.to_string();
                let plan = self.native_plan(&name)?;
                let target = plan.target();
                let ret = self.emit_native(plan, args)?;
                let labels = self.call_type_args(type_args)?;
                self.record_call(CallKind::Native, name, target, labels, None);
                Ok(ret)
            }
            CallTarget::External { name } => Err(format!(
                "wasm TIR: unresolved external call target '{}'",
                self.name(*name).unwrap_or("<invalid name>")
            )),
        }
    }

    fn emit_native(&mut self, plan: NativeCall, args: &[TirExpr]) -> Result<WTy, String> {
        use Instruction::*;
        match plan {
            NativeCall::Host { index, ret, .. } => {
                for arg in args {
                    self.emit_expr(arg)?;
                }
                self.push(Call(index));
                Ok(ret)
            }
            NativeCall::Println => {
                if args.is_empty() {
                    return Err("wasm TIR: println expects one argument".to_string());
                }
                let ty = self.emit_expr(&args[0])?;
                let host = match ty {
                    WTy::Str => "print",
                    WTy::I64 => "print_i64",
                    WTy::F64 => "print_f64",
                    WTy::Bool => "print_bool",
                    WTy::Char => "print_char",
                    WTy::Handle => "dyn_to_string",
                    WTy::Void | WTy::Unknown => {
                        return Err("wasm TIR: unsupported println argument type".to_string())
                    }
                };
                self.call_host(host)?;
                if ty == WTy::Handle {
                    self.call_host("print")?;
                }
                Ok(WTy::Void)
            }
            NativeCall::Print => {
                if args.is_empty() {
                    return Err("wasm TIR: print expects one argument".to_string());
                }
                let ty = self.emit_expr(&args[0])?;
                self.coerce_to_str(ty)?;
                self.call_host("print_raw")?;
                Ok(WTy::Void)
            }
            NativeCall::Math { index } => {
                for arg in args {
                    let ty = self.emit_expr(arg)?;
                    self.coerce(ty, WTy::F64)?;
                }
                self.push(Call(index));
                Ok(WTy::F64)
            }
            NativeCall::StrLen => {
                if args.is_empty() {
                    return Err("wasm TIR: String.length expects one argument".to_string());
                }
                let ty = self.emit_expr(&args[0])?;
                self.coerce(ty, WTy::Str)?;
                self.call_host("str_len")?;
                self.push(I64ExtendI32U);
                Ok(WTy::I64)
            }
            NativeCall::StrEq => {
                if args.len() < 2 {
                    return Err("wasm TIR: String.equals expects two arguments".to_string());
                }
                let a = self.emit_expr(&args[0])?;
                self.coerce(a, WTy::Str)?;
                let b = self.emit_expr(&args[1])?;
                self.coerce(b, WTy::Str)?;
                self.call_host("str_eq")?;
                Ok(WTy::Bool)
            }
        }
    }

    fn emit_field(
        &mut self,
        object: &TirExpr,
        field: NameId,
        slot: Option<u16>,
        value: Option<&TirExpr>,
    ) -> Result<WTy, String> {
        let (resolved, ty) = self.field_plan(field, slot)?;
        let name = self.name(field)?.to_string();
        let object_ty = self.emit_expr(object)?;
        if object_ty != WTy::Handle {
            return Err(format!(
                "wasm TIR: field '{}' requires an object receiver",
                name
            ));
        }
        let offset = self.intern_string(&name);
        self.i32c(offset);
        let host = if value.is_some() { field_setter(ty) } else { field_getter(ty) };
        if let Some(expression) = value {
            let from = self.emit_expr(expression)?;
            self.coerce(from, ty)?;
            self.call_host(host)?;
            self.shape.field_accesses.push(FieldAccessShape {
                field: name,
                slot: resolved.slot,
                offset: resolved.offset,
                value: value_kind(ty),
                is_set: true,
            });
            Ok(WTy::Void)
        } else {
            self.call_host(host)?;
            self.shape.field_accesses.push(FieldAccessShape {
                field: name,
                slot: resolved.slot,
                offset: resolved.offset,
                value: value_kind(ty),
                is_set: false,
            });
            Ok(ty)
        }
    }

    fn emit_new(&mut self, class: NameId, args: &[TirExpr]) -> Result<WTy, String> {
        let class_name = self.name(class)?.to_string();
        // Collections have no TIR class layout; they lower to host constructors.
        let collection_host = match class_name.as_str() {
            "ArrayList" => Some("list_new"),
            "HashMap" => Some("map_new"),
            _ => None,
        };
        if let Some(host) = collection_host {
            self.call_host(host)?;
            return Ok(WTy::Handle);
        }
        if !self.program.classes.iter().any(|layout| layout.name == class) {
            return Err(format!("wasm TIR: unknown class '{}'", class_name));
        }
        let offset = self.intern_string(&class_name);
        self.i32c(offset);
        self.call_host("obj_named_new")?;
        let handle = self.add_local(WTy::Handle);
        self.push(Instruction::LocalSet(handle));

        let ctor_name = format!("{}.init", class_name);
        if let Some(plan) = self.funcs.iter().find(|f| f.name == ctor_name).cloned() {
            self.push(Instruction::LocalGet(handle));
            let ctor_params: Vec<WTy> = plan.params.iter().skip(1).copied().collect();
            self.compile_args(args, &ctor_params)?;
            self.push(Instruction::Call(plan.index));
            if plan.ret != WTy::Void {
                self.push(Instruction::Drop);
            }
            self.record_call(
                CallKind::Direct,
                ctor_name.clone(),
                ctor_name,
                Vec::new(),
                None,
            );
        }
        self.push(Instruction::LocalGet(handle));
        Ok(WTy::Handle)
    }
}

/// Lower a TIR program to a WebAssembly binary, returning the structural shape
/// alongside the bytes so callers can inspect what was emitted.
pub fn lower_tir_to_wasm(program: &TirProgram) -> Result<(Vec<u8>, TirWasmShape), String> {
    Emitter::new(program)?.compile_checked()
}

/// Lower a TIR program to a WebAssembly binary.
pub fn compile_tir_to_wasm(program: &TirProgram) -> Result<Vec<u8>, String> {
    lower_tir_to_wasm(program).map(|(bytes, _)| bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::ast::{Access, ClassMember, FieldDecl, FnDecl, MethodDecl, Param, Span, Type};
    use crate::tir::{
        lower, Capabilities, ClassLayout, FieldSlot, TirFunction, TirGlobal, TirParam, VTable,
        VTableSlot,
    };

    const MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

    fn span() -> Span {
        Span::new(1, 1)
    }

    fn named_type(names: &mut NameInterner, name: &str) -> TirType {
        TirType::Named { name: names.intern(name).unwrap(), params: vec![] }
    }

    /// A small TIR program assembled by hand: a main function, a generic-free
    /// helper, a class with one field slot, and a one-entry vtable.
    fn hand_built() -> TirProgram {
        let mut names = NameInterner::new();
        let main = names.intern("main").unwrap();
        let helper = names.intern("helper").unwrap();
        let point = names.intern("Point").unwrap();
        let x = names.intern("x").unwrap();
        let area = names.intern("area").unwrap();
        let area_fn = names.intern("Point.area").unwrap();
        let value = names.intern("value").unwrap();
        let counter = names.intern("counter").unwrap();
        let int_ty = named_type(&mut names, "int");
        let double_ty = named_type(&mut names, "double");

        let main_fn = TirFunction {
            id: FunctionId::new(0),
            name: main,
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![
                TirStmt::Let {
                    name: value,
                    typ: Some(int_ty.clone()),
                    init: Some(TirExpr::Literal(Literal::Int(42), span())),
                    mutable: false,
                    span: span(),
                },
                TirStmt::Expr(TirExpr::Call {
                    target: CallTarget::Direct(FunctionId::new(2)),
                    type_args: vec![],
                    args: vec![TirExpr::Var(value, span())],
                    span: span(),
                }),
                TirStmt::Expr(TirExpr::Field {
                    object: Box::new(TirExpr::New { class: point, args: vec![], span: span() }),
                    field: x,
                    slot: Some(0),
                    span: span(),
                }),
                TirStmt::Expr(TirExpr::Call {
                    target: CallTarget::Virtual { vtable: VTableId::new(0), slot: 0 },
                    type_args: vec![],
                    args: vec![TirExpr::New { class: point, args: vec![], span: span() }],
                    span: span(),
                }),
            ],
            capability: Capabilities::IO,
            simd: SimdHint::None,
            is_public: true,
        };
        let area_function = TirFunction {
            id: FunctionId::new(1),
            name: area_fn,
            type_params: vec![],
            params: vec![],
            return_type: Some(double_ty),
            body: vec![TirStmt::Return(Some(TirExpr::Literal(Literal::Float(3.5), span())))] ,
            capability: Capabilities::NONE,
            simd: SimdHint::None,
            is_public: true,
        };
        let helper_fn = TirFunction {
            id: FunctionId::new(2),
            name: helper,
            type_params: vec![],
            params: vec![TirParam { name: value, typ: int_ty.clone() }],
            return_type: None,
            body: vec![TirStmt::Expr(TirExpr::Unary {
                op: UnOp::Neg,
                operand: Box::new(TirExpr::Var(value, span())),
                span: span(),
            })],
            capability: Capabilities::NONE,
            simd: SimdHint::None,
            is_public: false,
        };

        TirProgram {
            names,
            functions: vec![main_fn, area_function, helper_fn],
            globals: vec![TirGlobal {
                name: counter,
                typ: None,
                init: None,
                is_const: false,
                is_public: false,
            }],
            classes: vec![ClassLayout {
                name: point,
                parent: None,
                fields: vec![FieldSlot { name: x, typ: int_ty, slot: 0, offset: 8 }],
                total_slots: 1,
                size_bytes: 16,
                vtable: VTableId::new(0),
            }],
            vtables: vec![VTable {
                class: point,
                entries: vec![VTableSlot { method: area, target: FunctionId::new(1) }],
            }],
        }
    }

    #[test]
    fn emits_binary_with_expected_shape() {
        let program = hand_built();
        let (bytes, shape) = lower_tir_to_wasm(&program).unwrap();
        assert_eq!(&bytes[..4], &MAGIC);
        assert_eq!(&bytes[4..8], &[0x01u8, 0x00, 0x00, 0x00]);
        assert_eq!(
            shape.functions.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            vec!["main", "Point.area", "helper"]
        );
        assert_eq!(shape.functions[0].capability, Capabilities::IO.bits());
        assert_eq!(shape.functions[0].simd, "none");
        assert!(!shape.functions[2].is_public);

        assert_eq!(
            shape.calls,
            vec![
                CallShape {
                    kind: CallKind::Direct,
                    name: "helper".to_string(),
                    target: "helper".to_string(),
                    type_args: vec![],
                    slot: None,
                },
                CallShape {
                    kind: CallKind::Virtual,
                    name: "area".to_string(),
                    target: "Point.area".to_string(),
                    type_args: vec![],
                    slot: Some(0),
                },
            ]
        );
        assert_eq!(
            shape.field_accesses,
            vec![FieldAccessShape {
                field: "x".to_string(),
                slot: 0,
                offset: 8,
                value: WasmValueKind::Int,
                is_set: false,
            }]
        );
        assert!(shape.exports.contains(&"main".to_string()));
        assert!(shape.string_literals.contains(&"x".to_string()));
    }

    #[test]
    fn compares_tir_derived_shape_from_ast_program() {
        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                class_decl(),
                ast::Declaration::Function(FnDecl {
                    access: Access::Public,
                    name: "main".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(Type::simple("void")),
                    body: vec![
                        ast::Stmt::Expr(ast::Expr::Call(
                            Box::new(ast::Expr::Identifier("helper".to_string(), span())),
                            vec![],
                            vec![],
                            span(),
                        )),
                        ast::Stmt::Expr(ast::Expr::MemberAccess(
                            Box::new(ast::Expr::New(Type::simple("Point"), vec![], span())),
                            "x".to_string(),
                            span(),
                        )),
                    ],
                    sugar: false,
                    where_clause: vec![],
                    span: span(),
                }),
                ast::Declaration::Function(FnDecl {
                    access: Access::Private,
                    name: "helper".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(Type::simple("void")),
                    body: vec![],
                    sugar: false,
                    where_clause: vec![],
                    span: span(),
                }),
            ],
        };

        let tir = lower(&program).unwrap();
        let (bytes, shape) = lower_tir_to_wasm(&tir).unwrap();
        assert_eq!(&bytes[..4], &MAGIC);
        assert_eq!(shape.functions.len(), 3);
        assert!(shape.functions.iter().any(|f| f.name == "main"));
        assert!(shape.calls.iter().any(|c| {
            c.kind == CallKind::Direct && c.name == "helper" && c.target == "helper"
        }));
        assert!(shape.field_accesses.iter().any(|f| {
            f.field == "x" && f.slot == 0 && f.value == WasmValueKind::Int
        }));
    }

    fn class_decl() -> ast::Declaration {
        ast::Declaration::Class(ast::ClassDecl {
            name: "Point".to_string(),
            type_params: vec![],
            parent: None,
            ifaces: vec![],
            members: vec![
                ClassMember::Field(FieldDecl {
                    access: Access::Public,
                    name: "x".to_string(),
                    typ: Type::simple("int"),
                    init: None,
                    span: span(),
                }),
                ClassMember::Method(MethodDecl {
                    access: Access::Public,
                    name: "area".to_string(),
                    type_params: vec![],
                    params: vec![Param { name: "this".to_string(), typ: Type::simple("Point") }],
                    return_type: Some(Type::simple("double")),
                    body: vec![],
                    where_clause: vec![],
                    span: span(),
                }),
            ],
            span: span(),
        })
    }

    #[test]
    fn rejects_unmodeled_external_call() {
        let mut program = hand_built();
        let mut names = std::mem::take(&mut program.names);
        let missing = names.intern("missing").unwrap();
        program.names = names;
        if let TirStmt::Expr(TirExpr::Call { target, .. }) = &mut program.functions[0].body[1] {
            *target = CallTarget::External { name: missing };
        }
        let error = lower_tir_to_wasm(&program).unwrap_err();
        assert!(error.contains("unresolved external call target"), "{}", error);
    }

    #[test]
    fn rejects_unmodeled_for_loop() {
        let mut program = hand_built();
        let mut names = std::mem::take(&mut program.names);
        let item = names.intern("item").unwrap();
        program.names = names;
        program.functions[0].body.push(TirStmt::For {
            var: item,
            iterable: TirExpr::Unit(span()),
            body: vec![],
            simd: SimdHint::None,
            span: span(),
        });
        let error = lower_tir_to_wasm(&program).unwrap_err();
        assert!(error.contains("for-in loops"), "{}", error);
    }

    #[test]
    fn rejects_unresolved_field_slot() {
        let mut program = hand_built();
        if let TirStmt::Expr(TirExpr::Field { slot, .. }) = &mut program.functions[0].body[2] {
            *slot = Some(7);
        }
        let error = lower_tir_to_wasm(&program).unwrap_err();
        assert!(error.contains("no class field slot"), "{}", error);
    }

    #[test]
    fn records_monomorphized_type_arguments() {
        let mut program = hand_built();
        let int = program.names.intern("int").unwrap();
        let int_ty = TirType::Named { name: int, params: vec![] };
        if let TirStmt::Expr(TirExpr::Call { type_args, .. }) = &mut program.functions[0].body[1] {
            *type_args = vec![int_ty];
        }
        let (_, shape) = lower_tir_to_wasm(&program).unwrap();
        assert_eq!(shape.calls[0].type_args, vec!["int".to_string()]);
    }

    #[test]
    fn rejects_unmonomorphized_generic_call() {
        let mut program = hand_built();
        let type_param = program.names.intern("T").unwrap();
        program.functions[2].type_params = vec![type_param];
        let error = lower_tir_to_wasm(&program).unwrap_err();
        assert!(error.contains("monomorphized"), "{}", error);
    }
}
