//! Structural parity harness for the legacy bytecode lowering and TIR lowering.
//!
//! Both paths lower the same [`ast::Program`]. This module compares the four
//! projections backends depend on: function names, field slots, call targets,
//! and capability flags. It only observes the two lowerings; it does not run a
//! backend or change how either one lowers a program.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ast;
use crate::bytecode::{self, Chunk, CompiledProgram, OpCode};

use super::types::{
    CallTarget, Capabilities, TirExpr, TirProgram, TirStmt, VTableId,
};

/// The structural projection that differed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DifferenceKind {
    FunctionName,
    FieldSlot,
    CallTarget,
    Capability,
}

/// One discrepancy between the legacy lowering and the TIR lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    pub kind: DifferenceKind,
    pub subject: String,
    pub legacy: String,
    pub tir: String,
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}: {}: legacy={}, tir={}",
            self.kind, self.subject, self.legacy, self.tir
        )
    }
}

/// All discrepancies found in one comparison.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComparisonReport {
    pub differences: Vec<Difference>,
}

impl ComparisonReport {
    fn push(
        &mut self,
        kind: DifferenceKind,
        subject: impl Into<String>,
        legacy: impl Into<String>,
        tir: impl Into<String>,
    ) {
        self.differences.push(Difference {
            kind,
            subject: subject.into(),
            legacy: legacy.into(),
            tir: tir.into(),
        });
    }

    pub fn is_parity(&self) -> bool {
        self.differences.is_empty()
    }

    pub fn has_differences(&self) -> bool {
        !self.is_parity()
    }

    pub fn len(&self) -> usize {
        self.differences.len()
    }

    pub fn is_empty(&self) -> bool {
        self.differences.is_empty()
    }

    pub fn of_kind(&self, kind: DifferenceKind) -> impl Iterator<Item = &Difference> {
        self.differences.iter().filter(move |item| item.kind == kind)
    }
}

impl fmt::Display for ComparisonReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.differences.is_empty() {
            return write!(f, "parity: legacy and TIR lowerings agree");
        }
        writeln!(f, "{} structural differences:", self.differences.len())?;
        for difference in &self.differences {
            writeln!(f, "  {}", difference)?;
        }
        Ok(())
    }
}

/// Lower `program` with both paths and compare their structural projections.
///
/// Returns an error when either lowering rejects the program; a rejected
/// program has no meaningful structural comparison.
pub fn compare(program: &ast::Program) -> Result<ComparisonReport, String> {
    let mut compiler = bytecode::Compiler::new();
    let legacy = compiler
        .compile(program)
        .map_err(|error| format!("legacy lowering failed: {}", error))?;
    let tir = super::lower(program).map_err(|error| format!("TIR lowering failed: {}", error))?;
    Ok(compare_programs(&legacy, &tir))
}

/// Compare two already-lowered programs. Useful when the caller keeps both
/// lowerings around for a differential run.
pub fn compare_programs(legacy: &CompiledProgram, tir: &TirProgram) -> ComparisonReport {
    let mut report = ComparisonReport::default();
    compare_function_names(legacy, tir, &mut report);
    compare_field_slots(legacy, tir, &mut report);

    let legacy_calls = legacy_call_targets(legacy);
    let tir_calls = tir_call_targets(tir);
    compare_call_targets(&legacy_calls, &tir_calls, &mut report);
    compare_capabilities(&legacy_calls, tir, &mut report);

    report
}

/// The legacy compiler's reserved top-level chunk; TIR has no equivalent.
const MAIN_CHUNK: &str = "<main>";

/// Capability bits that have a comparable signal in both lowerings. TIR marks
/// `MUTATION` on assignment expressions, while legacy bytecode emits the same
/// store opcodes for `let` initializers, so that bit is excluded here.
const PARITY_CAPABILITIES: u16 = Capabilities::IO.bits()
    | Capabilities::FILE_SYSTEM.bits()
    | Capabilities::NETWORK.bits()
    | Capabilities::TIME.bits()
    | Capabilities::RANDOM.bits()
    | Capabilities::NATIVE.bits();

fn compare_function_names(
    legacy: &CompiledProgram,
    tir: &TirProgram,
    report: &mut ComparisonReport,
) {
    let mut legacy_names: Vec<&str> = legacy
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .filter(|name| *name != MAIN_CHUNK)
        .collect();
    let mut tir_names: Vec<&str> = tir
        .functions
        .iter()
        .filter_map(|function| tir.name(function.name))
        .collect();
    legacy_names.sort_unstable();
    tir_names.sort_unstable();

    let (absent_in_tir, extra_in_tir) = merge_sorted(&legacy_names, &tir_names);
    for name in absent_in_tir {
        report.push(
            DifferenceKind::FunctionName,
            format!("function {}", name),
            "present",
            "missing",
        );
    }
    for name in extra_in_tir {
        report.push(
            DifferenceKind::FunctionName,
            format!("function {}", name),
            "missing",
            "present",
        );
    }
}

/// Compare each legacy class's own field order with TIR's field slots. Legacy
/// stores fields in declaration order; TIR numbers them after the inherited
/// slots, so the expected slot is the parent's total slot count plus the
/// declaration index.
fn compare_field_slots(legacy: &CompiledProgram, tir: &TirProgram, report: &mut ComparisonReport) {
    let mut tir_classes: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, layout) in tir.classes.iter().enumerate() {
        if let Some(name) = tir.name(layout.name) {
            tir_classes.insert(name, index);
        }
    }

    let mut seen_classes: BTreeSet<&str> = BTreeSet::new();
    for class in &legacy.classes {
        seen_classes.insert(class.name.as_str());
        let Some(&index) = tir_classes.get(class.name.as_str()) else {
            report.push(
                DifferenceKind::FieldSlot,
                format!("class {}", class.name),
                "present",
                "missing",
            );
            continue;
        };

        let layout = &tir.classes[index];
        let base = layout
            .parent
            .map(|parent| u32::from(tir.class(parent).total_slots))
            .unwrap_or(0);

        for (own_index, field) in class.fields.iter().enumerate() {
            let subject = format!("{}.{}", class.name, field.name);
            match layout.fields.get(own_index) {
                None => report.push(
                    DifferenceKind::FieldSlot,
                    subject,
                    format!("slot {}", base + own_index as u32),
                    "missing",
                ),
                Some(slot) => {
                    let tir_name = tir.name(slot.name).unwrap_or("<unknown name>");
                    if tir_name != field.name.as_str() {
                        report.push(
                            DifferenceKind::FieldSlot,
                            subject.clone(),
                            field.name.as_str(),
                            tir_name,
                        );
                    }
                    let expected = base + own_index as u32;
                    if u32::from(slot.slot) != expected {
                        report.push(
                            DifferenceKind::FieldSlot,
                            subject,
                            format!("slot {}", expected),
                            format!("slot {}", slot.slot),
                        );
                    }
                }
            }
        }

        for slot in layout.fields.iter().skip(class.fields.len()) {
            let name = tir.name(slot.name).unwrap_or("<unknown name>");
            report.push(
                DifferenceKind::FieldSlot,
                format!("{}.{}", class.name, name),
                "missing",
                format!("slot {}", slot.slot),
            );
        }
    }

    for name in tir_classes.keys() {
        if !seen_classes.contains(name) {
            report.push(
                DifferenceKind::FieldSlot,
                format!("class {}", name),
                "missing",
                "present",
            );
        }
    }
}

/// Function name to sorted list of normalized call-target labels.
type CallTargetMap = BTreeMap<String, Vec<String>>;

fn legacy_call_targets(program: &CompiledProgram) -> CallTargetMap {
    let mut result = CallTargetMap::new();
    for function in &program.functions {
        if function.name == MAIN_CHUNK {
            continue;
        }
        result.insert(
            function.name.clone(),
            decode_legacy_targets(program, &function.chunk),
        );
    }
    result
}

/// Decode every call-like instruction in a legacy chunk into a canonical
/// label. The prefix records the dispatch kind so a direct call is not
/// mistaken for a virtual or native one.
fn decode_legacy_targets(program: &CompiledProgram, chunk: &Chunk) -> Vec<String> {
    let mut targets = Vec::new();
    let mut ip = 0usize;
    while ip < chunk.code.len() {
        let Ok(opcode) = OpCode::try_from(chunk.code[ip]) else {
            break;
        };
        ip += 1;
        let operand_bytes = opcode.operand_size();
        if ip + operand_bytes > chunk.code.len() {
            break;
        }

        match opcode {
            OpCode::CALL | OpCode::TAIL_CALL | OpCode::CALL_SUPER => {
                let index = chunk.read_u16(ip) as usize;
                let name = program
                    .functions
                    .get(index)
                    .map(|function| function.name.as_str())
                    .unwrap_or("<invalid index>");
                targets.push(format!("fn:{}", name));
            }
            OpCode::CALL_NATIVE => {
                let index = chunk.read_u16(ip) as usize;
                let name = program
                    .native_names
                    .get(index)
                    .map(String::as_str)
                    .unwrap_or("<invalid index>");
                targets.push(format!("native:{}", name));
            }
            OpCode::STATIC_CALL => {
                let class_index = chunk.read_u16(ip) as usize;
                let method_index = chunk.read_u16(ip + 2) as usize;
                let class = chunk
                    .strings
                    .get(class_index)
                    .map(String::as_str)
                    .unwrap_or("<invalid index>");
                let method = chunk
                    .strings
                    .get(method_index)
                    .map(String::as_str)
                    .unwrap_or("<invalid index>");
                targets.push(format!("native:{}.{}", class, method));
            }
            OpCode::INVOKE_VIRTUAL | OpCode::INVOKE_OPERATOR => {
                let method_index = chunk.read_u16(ip) as usize;
                let method = chunk
                    .strings
                    .get(method_index)
                    .map(String::as_str)
                    .unwrap_or("<invalid index>");
                targets.push(format!("virtual:{}", method));
            }
            OpCode::CALL_CLOSURE => targets.push("closure:<dynamic>".to_string()),
            _ => {}
        }

        ip += operand_bytes;
    }
    targets
}

fn tir_call_targets(program: &TirProgram) -> CallTargetMap {
    let mut result = CallTargetMap::new();
    for function in &program.functions {
        let name = program.name(function.name).unwrap_or("<unknown name>").to_string();
        let mut targets = Vec::new();
        collect_stmt_targets(&function.body, program, &mut targets);
        result.insert(name, targets);
    }
    result
}

fn collect_stmt_targets(statements: &[TirStmt], program: &TirProgram, out: &mut Vec<String>) {
    for statement in statements {
        match statement {
            TirStmt::Block(body) => collect_stmt_targets(body, program, out),
            TirStmt::Expr(expression) => collect_expr_targets(expression, program, out),
            TirStmt::Let { init, .. } => {
                if let Some(expression) = init {
                    collect_expr_targets(expression, program, out);
                }
            }
            TirStmt::Return(value) => {
                if let Some(expression) = value {
                    collect_expr_targets(expression, program, out);
                }
            }
            TirStmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                collect_expr_targets(cond, program, out);
                collect_stmt_targets(then_branch, program, out);
                if let Some(body) = else_branch {
                    collect_stmt_targets(body, program, out);
                }
            }
            TirStmt::While { cond, body, .. } | TirStmt::For { iterable: cond, body, .. } => {
                collect_expr_targets(cond, program, out);
                collect_stmt_targets(body, program, out);
            }
            TirStmt::Break | TirStmt::Continue => {}
        }
    }
}

fn collect_expr_targets(expression: &TirExpr, program: &TirProgram, out: &mut Vec<String>) {
    match expression {
        TirExpr::Call {
            target, args, ..
        } => {
            out.push(tir_target_label(program, target));
            for argument in args {
                collect_expr_targets(argument, program, out);
            }
        }
        TirExpr::Unary { operand, .. }
        | TirExpr::Cast { value: operand, .. }
        | TirExpr::Is { value: operand, .. }
        | TirExpr::Ref { value: operand, .. } => collect_expr_targets(operand, program, out),
        TirExpr::Binary { left, right, .. } | TirExpr::Assign { target: left, value: right, .. } => {
            collect_expr_targets(left, program, out);
            collect_expr_targets(right, program, out);
        }
        TirExpr::Field { object, .. } => collect_expr_targets(object, program, out),
        TirExpr::New { args, .. } => {
            for argument in args {
                collect_expr_targets(argument, program, out);
            }
        }
        TirExpr::Tuple { items, .. } => {
            for item in items {
                collect_expr_targets(item, program, out);
            }
        }
        TirExpr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            collect_expr_targets(condition, program, out);
            collect_expr_targets(then_expr, program, out);
            collect_expr_targets(else_expr, program, out);
        }
        TirExpr::Literal(_, _)
        | TirExpr::Var(_, _)
        | TirExpr::This(_)
        | TirExpr::Unit(_) => {}
    }
}

fn tir_target_label(program: &TirProgram, target: &CallTarget) -> String {
    match target {
        CallTarget::Direct(id) => format!(
            "fn:{}",
            name_or(program.name(program.function(*id).name))
        ),
        CallTarget::Native { name } => format!("native:{}", name_or(program.name(*name))),
        CallTarget::External { name } => format!("external:{}", name_or(program.name(*name))),
        CallTarget::Virtual { vtable, slot } => {
            let method = vtable_method(program, *vtable, *slot);
            format!("virtual:{}", name_or(method))
        }
    }
}

fn vtable_method<'a>(program: &'a TirProgram, vtable: VTableId, slot: u16) -> Option<&'a str> {
    program
        .vtable(vtable)
        .entries
        .get(slot as usize)
        .and_then(|entry| program.name(entry.method))
}

fn name_or(name: Option<&str>) -> &str {
    name.unwrap_or("<unknown name>")
}

fn compare_call_targets(
    legacy: &CallTargetMap,
    tir: &CallTargetMap,
    report: &mut ComparisonReport,
) {
    for (function, legacy_targets) in legacy {
        let Some(tir_targets) = tir.get(function) else {
            continue;
        };
        let mut left: Vec<&str> = legacy_targets.iter().map(String::as_str).collect();
        let mut right: Vec<&str> = tir_targets.iter().map(String::as_str).collect();
        left.sort_unstable();
        right.sort_unstable();

        let (absent_in_tir, extra_in_tir) = merge_sorted(&left, &right);
        for label in absent_in_tir {
            report.push(
                DifferenceKind::CallTarget,
                format!("{} calls", function),
                label,
                "missing",
            );
        }
        for label in extra_in_tir {
            report.push(
                DifferenceKind::CallTarget,
                format!("{} calls", function),
                "missing",
                label,
            );
        }
    }
}

fn compare_capabilities(legacy: &CallTargetMap, tir: &TirProgram, report: &mut ComparisonReport) {
    for function in &tir.functions {
        let Some(name) = tir.name(function.name) else {
            continue;
        };
        let Some(targets) = legacy.get(name) else {
            continue;
        };
        let derived = capabilities_from_targets(targets).bits() & PARITY_CAPABILITIES;
        let declared = function.capability.bits() & PARITY_CAPABILITIES;
        if derived != declared {
            report.push(
                DifferenceKind::Capability,
                format!("function {} capabilities", name),
                format_capabilities(derived),
                format_capabilities(declared),
            );
        }
    }
}

/// Derive the legacy side's capability bits from the names of its call
/// targets. Only native and external names carry effects; direct and virtual
/// calls resolve to user code whose own body is inspected separately.
fn capabilities_from_targets(targets: &[String]) -> Capabilities {
    let mut capabilities = Capabilities::NONE;
    for target in targets {
        let Some((kind, name)) = target.split_once(':') else {
            continue;
        };
        if kind != "native" && kind != "external" {
            continue;
        }
        note_effects(name, &mut capabilities);
    }
    capabilities
}

fn note_effects(name: &str, capabilities: &mut Capabilities) {
    if name == "print" || name == "println" || name.starts_with("IO_") {
        capabilities.insert(Capabilities::IO);
    }
    if name.starts_with("File_") || name.starts_with("Dir_") || name.starts_with("Path_") {
        capabilities.insert(Capabilities::FILE_SYSTEM);
    }
    if name.starts_with("Net_") {
        capabilities.insert(Capabilities::NETWORK);
    }
    if name.starts_with("Time_") {
        capabilities.insert(Capabilities::TIME);
    }
    if name.starts_with("Random_") {
        capabilities.insert(Capabilities::RANDOM);
    }
    if name.contains('_') || name.contains('.') {
        capabilities.insert(Capabilities::NATIVE);
    }
}

fn format_capabilities(bits: u16) -> String {
    if bits == 0 {
        return "pure".to_string();
    }
    let mut names = Vec::new();
    for (flag, label) in [
        (Capabilities::IO, "IO"),
        (Capabilities::FILE_SYSTEM, "FILE_SYSTEM"),
        (Capabilities::NETWORK, "NETWORK"),
        (Capabilities::TIME, "TIME"),
        (Capabilities::RANDOM, "RANDOM"),
        (Capabilities::NATIVE, "NATIVE"),
    ] {
        if bits & flag.bits() != 0 {
            names.push(label);
        }
    }
    names.join("|")
}

/// Return values present only in `legacy` and only in `tir` for two sorted
/// slices, ignoring equal values (including duplicates).
fn merge_sorted<'a>(legacy: &[&'a str], tir: &[&'a str]) -> (Vec<&'a str>, Vec<&'a str>) {
    let (mut left, mut right) = (0usize, 0usize);
    let (mut absent_in_tir, mut extra_in_tir) = (Vec::new(), Vec::new());
    while left < legacy.len() && right < tir.len() {
        match legacy[left].cmp(&tir[right]) {
            Ordering::Less => {
                absent_in_tir.push(legacy[left]);
                left += 1;
            }
            Ordering::Greater => {
                extra_in_tir.push(tir[right]);
                right += 1;
            }
            Ordering::Equal => {
                left += 1;
                right += 1;
            }
        }
    }
    absent_in_tir.extend_from_slice(&legacy[left..]);
    extra_in_tir.extend_from_slice(&tir[right..]);
    (absent_in_tir, extra_in_tir)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::ast::{Access, ClassMember, FieldDecl, Literal, MethodDecl, Param, Span, Type};
    use crate::bytecode::frame::{ClassDef, FieldDef, FunctionDef};
    use crate::bytecode::OpCode as LegacyOpCode;
    use crate::tir::types::{FunctionId, SimdHint, TirType};

    fn span() -> Span {
        Span::new(1, 1)
    }

    fn function(name: &str, body: ast::Block) -> ast::Declaration {
        ast::Declaration::Function(ast::FnDecl {
            access: Access::Public,
            name: name.to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(Type::simple("void")),
            body,
            sugar: false,
            where_clause: vec![],
            span: span(),
        })
    }

    fn call(callee: ast::Expr, args: Vec<ast::Expr>) -> ast::Expr {
        ast::Expr::Call(Box::new(callee), args, vec![], span())
    }

    fn identifier(name: &str) -> ast::Expr {
        ast::Expr::Identifier(name.to_string(), span())
    }

    fn string(value: &str) -> ast::Expr {
        ast::Expr::Literal(Literal::String(value.to_string()), span())
    }

    fn field(name: &str, typ: Type) -> ClassMember {
        ClassMember::Field(FieldDecl {
            access: Access::Public,
            name: name.to_string(),
            typ,
            init: None,
            span: span(),
        })
    }

    fn method(name: &str, return_type: Type) -> ClassMember {
        ClassMember::Method(MethodDecl {
            access: Access::Public,
            name: name.to_string(),
            type_params: vec![],
            params: vec![Param {
                name: "this".to_string(),
                typ: Type::simple("Point"),
            }],
            return_type: Some(return_type),
            body: vec![],
            where_clause: vec![],
            span: span(),
        })
    }

    fn point() -> ast::Declaration {
        ast::Declaration::Class(ast::ClassDecl {
            name: "Point".to_string(),
            type_params: vec![],
            parent: None,
            ifaces: vec![],
            members: vec![
                field("x", Type::simple("int")),
                field("y", Type::simple("double")),
                method("area", Type::simple("double")),
            ],
            span: span(),
        })
    }

    #[test]
    fn reports_parity_for_supported_program() {
        let virtual_call = ast::Expr::MemberAccess(
            Box::new(ast::Expr::New(Type::simple("Point"), vec![], span())),
            "area".to_string(),
            span(),
        );
        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                function("helper", vec![]),
                point(),
                function(
                    "main",
                    vec![
                        ast::Stmt::Expr(call(identifier("helper"), vec![])),
                        ast::Stmt::Expr(call(virtual_call, vec![])),
                    ],
                ),
            ],
        };

        let report = compare(&program).expect("both lowerings should accept this program");
        assert!(
            report.is_parity(),
            "unexpected differences:\n{}",
            report
        );
    }

    #[test]
    fn reports_builtin_dispatch_kind_and_matching_capability() {
        let program = ast::Program {
            imports: vec![],
            declarations: vec![function(
                "main",
                vec![ast::Stmt::Expr(call(identifier("println"), vec![string("hi")]))],
            )],
        };

        let report = compare(&program).expect("both lowerings should accept this program");
        let targets: Vec<&Difference> = report.of_kind(DifferenceKind::CallTarget).collect();
        assert_eq!(targets.len(), 1, "unexpected report:\n{}", report);
        assert_eq!(targets[0].legacy, "native:println");
        assert_eq!(targets[0].tir, "external:println");
        assert_eq!(report.of_kind(DifferenceKind::Capability).count(), 0);
    }

    #[test]
    fn compares_manual_programs_for_every_difference_kind() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(LegacyOpCode::CALL, 1);
        chunk.write_u16(2, 1);
        chunk.write_u8(0, 1);
        chunk.write_opcode(LegacyOpCode::RET, 1);

        fn legacy_function(name: &str, chunk: Chunk) -> FunctionDef {
            FunctionDef {
                name: name.to_string(),
                arity: 0,
                chunk,
                is_method: false,
                is_constructor: false,
                local_count: 0,
                param_types: vec![],
            }
        }

        let legacy = CompiledProgram {
            functions: vec![
                legacy_function(MAIN_CHUNK, Chunk::new()),
                legacy_function("main", chunk),
                legacy_function("helper", Chunk::new()),
            ],
            classes: vec![ClassDef {
                name: "Point".to_string(),
                parent: None,
                fields: vec![FieldDef {
                    name: "x".to_string(),
                    has_init: false,
                }],
                methods: HashMap::new(),
                constructor: None,
                field_inits: vec![],
            }],
            enums: vec![],
            native_names: vec![],
            global_count: 0,
        };

        let mut names = NameInterner::new();
        let main_name = names.intern("main").unwrap();
        let tir = TirProgram {
            names,
            functions: vec![TirFunction {
                id: FunctionId::new(0),
                name: main_name,
                type_params: vec![],
                params: vec![],
                return_type: None,
                body: vec![],
                capability: Capabilities::IO,
                simd: SimdHint::None,
                is_public: true,
            }],
            globals: vec![],
            classes: vec![],
            vtables: vec![],
        };

        let report = compare_programs(&legacy, &tir);
        assert!(report.of_kind(DifferenceKind::FunctionName).any(|item| {
            item.subject == "function helper" && item.legacy == "present" && item.tir == "missing"
        }));
        assert!(report.of_kind(DifferenceKind::FieldSlot).any(|item| {
            item.subject == "class Point" && item.legacy == "present" && item.tir == "missing"
        }));
        assert!(report.of_kind(DifferenceKind::CallTarget).any(|item| {
            item.subject == "main calls" && item.legacy == "fn:helper" && item.tir == "missing"
        }));
        assert!(report.of_kind(DifferenceKind::Capability).any(|item| {
            item.subject == "function main capabilities"
                && item.legacy == "pure"
                && item.tir == "IO"
        }));
        assert!(report.has_differences());
        assert!(format!("{}", report).contains("structural differences"));
    }

    #[test]
    fn detects_wrong_tir_field_slot() {
        let legacy = CompiledProgram {
            functions: vec![legacy_program_chunk()],
            classes: vec![ClassDef {
                name: "Point".to_string(),
                parent: None,
                fields: vec![
                    FieldDef {
                        name: "x".to_string(),
                        has_init: false,
                    },
                    FieldDef {
                        name: "y".to_string(),
                        has_init: false,
                    },
                ],
                methods: HashMap::new(),
                constructor: None,
                field_inits: vec![],
            }],
            enums: vec![],
            native_names: vec![],
            global_count: 0,
        };

        let mut names = NameInterner::new();
        let class_name = names.intern("Point").unwrap();
        let x = names.intern("x").unwrap();
        let y = names.intern("y").unwrap();
        let int = names.intern("int").unwrap();
        let tir = TirProgram {
            names,
            functions: vec![],
            globals: vec![],
            classes: vec![crate::tir::types::ClassLayout {
                name: class_name,
                parent: None,
                fields: vec![
                    crate::tir::types::FieldSlot {
                        name: x,
                        typ: TirType::Named {
                            name: int,
                            params: vec![],
                        },
                        slot: 0,
                        offset: 8,
                    },
                    crate::tir::types::FieldSlot {
                        name: y,
                        typ: TirType::Named {
                            name: int,
                            params: vec![],
                        },
                        slot: 7,
                        offset: 12,
                    },
                ],
                total_slots: 2,
                size_bytes: 16,
                vtable: VTableId::new(0),
            }],
            vtables: vec![],
        };

        let report = compare_programs(&legacy, &tir);
        assert!(report.of_kind(DifferenceKind::FieldSlot).any(|item| {
            item.subject == "Point.y" && item.legacy == "slot 1" && item.tir == "slot 7"
        }));
    }

    fn legacy_program_chunk() -> FunctionDef {
        FunctionDef {
            name: MAIN_CHUNK.to_string(),
            arity: 0,
            chunk: Chunk::new(),
            is_method: false,
            is_constructor: false,
            local_count: 0,
            param_types: vec![],
        }
    }

    #[test]
    fn capability_projection_uses_call_names_only() {
        let targets = vec![
            "native:println".to_string(),
            "native:File_read".to_string(),
            "fn:helper".to_string(),
            "virtual:area".to_string(),
        ];
        let capabilities = capabilities_from_targets(&targets);
        assert!(capabilities.contains(Capabilities::IO));
        assert!(capabilities.contains(Capabilities::FILE_SYSTEM));
        assert!(capabilities.contains(Capabilities::NATIVE));
        assert!(!capabilities.contains(Capabilities::NETWORK));
        assert!(!capabilities.contains(Capabilities::MUTATION));
        assert_eq!(format_capabilities(0), "pure");
    }

    #[test]
    fn reports_lowering_failure_instead_of_partial_comparison() {
        let program = ast::Program {
            imports: vec![],
            declarations: vec![function(
                "main",
                vec![ast::Stmt::Throw(string("boom"), span())],
            )],
        };
        let error = compare(&program).expect_err("TIR rejects throw statements");
        assert!(
            error.contains("TIR lowering failed"),
            "unexpected error: {}",
            error
        );
    }

    #[test]
    fn merge_sorted_reports_only_unmatched_values() {
        let legacy = ["a", "b", "d", "d"];
        let tir = ["b", "c", "d"];
        let (absent, extra) = merge_sorted(&legacy, &tir);
        assert_eq!(absent, vec!["a", "d"]);
        assert_eq!(extra, vec!["c"]);
    }
}
