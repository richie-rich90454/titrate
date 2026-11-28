//! Closure-capture layout helpers for the WASM backend.
//!
//! A captured variable is stored in a host `ClosureEnv` object under its
//! source name, using the same `field_set_*` / `field_get_*` imports as class
//! fields. This module owns the capture plan (name plus physical type) and the
//! instruction sequences that populate and read an environment handle. It is
//! deliberately independent of the per-function emitter state so both the AST
//! lowering and the extended TIR path can share it.

use std::collections::HashMap;

use wasm_encoder::Instruction;

use super::runtime::import_index;
use super::WTy;

/// One captured variable's environment slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureField {
    pub name: String,
    pub ty: WTy,
}

/// Stable capture layout. Order is the declaration order of the captures,
/// with later duplicates of the same name discarded.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapturePlan {
    pub fields: Vec<CaptureField>,
}

fn setter(ty: WTy) -> &'static str {
    match ty {
        WTy::F64 => "field_set_f64",
        WTy::Bool => "field_set_bool",
        WTy::Str | WTy::Char => "field_set_str",
        WTy::Handle => "field_set_obj",
        WTy::I64 | WTy::Void | WTy::Unknown => "field_set_i64",
    }
}

fn getter(ty: WTy) -> &'static str {
    match ty {
        WTy::F64 => "field_get_f64",
        WTy::Bool => "field_get_bool",
        WTy::Str | WTy::Char => "field_get_str",
        WTy::Handle => "field_get_obj",
        WTy::I64 | WTy::Void | WTy::Unknown => "field_get_i64",
    }
}

impl CapturePlan {
    /// Build a plan from `(name, physical type)` pairs.
    pub fn new(captures: &[(String, WTy)]) -> Self {
        let mut fields: Vec<CaptureField> = Vec::with_capacity(captures.len());
        for (name, ty) in captures {
            if fields.iter().any(|field| field.name == *name) {
                continue;
            }
            fields.push(CaptureField { name: name.clone(), ty: *ty });
        }
        Self { fields }
    }

    /// Emit stores of each captured local into the environment handle.
    ///
    /// `values` maps a capture name to the local holding its value; `intern`
    /// returns the linear-memory pointer of the field-name string.
    pub fn emit_store<F>(
        &self,
        out: &mut Vec<Instruction<'static>>,
        env_local: u32,
        values: &HashMap<String, u32>,
        intern: &mut F,
    ) -> Result<(), String>
    where
        F: FnMut(&str) -> i32,
    {
        for field in &self.fields {
            let value = values.get(&field.name).copied().ok_or_else(|| {
                format!("wasm: cannot capture unknown local '{}'", field.name)
            })?;
            let index = import_index(setter(field.ty))
                .ok_or_else(|| format!("wasm: missing host import '{}'", setter(field.ty)))?;
            out.push(Instruction::LocalGet(env_local));
            out.push(Instruction::I32Const(intern(&field.name)));
            out.push(Instruction::LocalGet(value));
            out.push(Instruction::Call(index));
        }
        Ok(())
    }

    /// Emit a load of one captured field. Returns its physical type.
    pub fn emit_load<F>(
        &self,
        out: &mut Vec<Instruction<'static>>,
        env_local: u32,
        name: &str,
        intern: &mut F,
    ) -> Result<WTy, String>
    where
        F: FnMut(&str) -> i32,
    {
        let field = self
            .fields
            .iter()
            .find(|field| field.name == name)
            .ok_or_else(|| format!("wasm: '{}' is not a captured variable", name))?;
        let index = import_index(getter(field.ty))
            .ok_or_else(|| format!("wasm: missing host import '{}'", getter(field.ty)))?;
        out.push(Instruction::LocalGet(env_local));
        out.push(Instruction::I32Const(intern(name)));
        out.push(Instruction::Call(index));
        Ok(field.ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn captures() -> Vec<(String, WTy)> {
        vec![
            ("n".to_string(), WTy::I64),
            ("label".to_string(), WTy::Str),
            ("n".to_string(), WTy::F64),
        ]
    }

    #[test]
    fn plan_deduplicates_by_name_keeping_first_type() {
        let plan = CapturePlan::new(&captures());
        assert_eq!(plan.fields.len(), 2);
        assert_eq!(plan.fields[0], CaptureField { name: "n".to_string(), ty: WTy::I64 });
        assert_eq!(plan.fields[1].ty, WTy::Str);
    }

    #[test]
    fn stores_and_loads_use_matching_field_imports() {
        let plan = CapturePlan::new(&captures());
        let values = HashMap::from([("n".to_string(), 3u32), ("label".to_string(), 4u32)]);
        let mut store = Vec::new();
        plan.emit_store(&mut store, 0, &values, &mut |name| name.len() as i32).unwrap();
        assert_eq!(store.len(), 8);

        let mut load = Vec::new();
        let ty = plan.emit_load(&mut load, 0, "label", &mut |name| name.len() as i32).unwrap();
        assert_eq!(ty, WTy::Str);
        assert_eq!(load.len(), 3);
    }

    #[test]
    fn unknown_capture_is_an_error() {
        let plan = CapturePlan::new(&captures());
        let values = HashMap::new();
        let mut out = Vec::new();
        let error = plan.emit_store(&mut out, 0, &values, &mut |_| 0).unwrap_err();
        assert!(error.contains("unknown local"), "{}", error);
        let error = plan.emit_load(&mut out, 0, "missing", &mut |_| 0).unwrap_err();
        assert!(error.contains("not a captured variable"), "{}", error);
    }
}
