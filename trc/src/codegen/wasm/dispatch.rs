//! Interface / vtable dispatch helpers for the WASM backend.
//!
//! TIR records virtual calls as a vtable id plus a slot index. This module
//! resolves those ids against the shared layout tables and produces a small,
//! inspectable plan. The same plan can drive an `call_indirect` slot lookup or
//! a direct call when the vtable entry is already monomorphized. Missing
//! ids and slots are reported explicitly; no name is guessed.

use crate::tir::{FunctionId, TirProgram, VTableId};

/// A resolved virtual dispatch site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchPlan {
    pub class: String,
    pub method: String,
    pub target: FunctionId,
    pub vtable: VTableId,
    pub slot: u16,
}

/// Resolve one vtable slot to its class, method name and target function.
pub fn resolve(program: &TirProgram, vtable: VTableId, slot: u16) -> Result<DispatchPlan, String> {
    let table = program
        .vtables
        .get(vtable.as_u16() as usize)
        .ok_or_else(|| format!("wasm: invalid vtable id {}", vtable.as_u16()))?;
    let entry = table
        .entries
        .get(slot as usize)
        .ok_or_else(|| format!("wasm: vtable {} has no slot {}", vtable.as_u16(), slot))?;
    let class = program
        .name(table.class)
        .ok_or_else(|| format!("wasm: invalid vtable class id {}", table.class.as_u16()))?
        .to_string();
    let method = program
        .name(entry.method)
        .ok_or_else(|| format!("wasm: invalid method id {}", entry.method.as_u16()))?
        .to_string();
    Ok(DispatchPlan {
        class,
        method,
        target: entry.target,
        vtable,
        slot,
    })
}

/// Find the slot of a named method in a vtable.
pub fn find_slot(program: &TirProgram, vtable: VTableId, method: &str) -> Result<u16, String> {
    let table = program
        .vtables
        .get(vtable.as_u16() as usize)
        .ok_or_else(|| format!("wasm: invalid vtable id {}", vtable.as_u16()))?;
    for (index, entry) in table.entries.iter().enumerate() {
        if program.name(entry.method) == Some(method) {
            return u16::try_from(index)
                .map_err(|_| format!("wasm: vtable slot overflow for '{}'", method));
        }
    }
    Err(format!(
        "wasm: vtable {} has no method '{}'",
        vtable.as_u16(),
        method
    ))
}

/// Resolve a method name to a complete dispatch plan.
pub fn resolve_named(
    program: &TirProgram,
    vtable: VTableId,
    method: &str,
) -> Result<DispatchPlan, String> {
    let slot = find_slot(program, vtable, method)?;
    resolve(program, vtable, slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tir::{NameInterner, VTable, VTableSlot};

    fn program() -> TirProgram {
        let mut names = NameInterner::new();
        let shape = names.intern("Shape").unwrap();
        let area = names.intern("area").unwrap();
        TirProgram {
            names,
            functions: Vec::new(),
            globals: Vec::new(),
            classes: Vec::new(),
            vtables: vec![VTable {
                class: shape,
                entries: vec![VTableSlot { method: area, target: FunctionId::new(4) }],
            }],
        }
    }

    #[test]
    fn resolves_slots_and_names() {
        let program = program();
        let plan = resolve(&program, VTableId::new(0), 0).unwrap();
        assert_eq!(plan.class, "Shape");
        assert_eq!(plan.method, "area");
        assert_eq!(plan.target, FunctionId::new(4));
        assert_eq!(find_slot(&program, VTableId::new(0), "area"), Ok(0));
        assert_eq!(resolve_named(&program, VTableId::new(0), "area").unwrap(), plan);
    }

    #[test]
    fn missing_entries_are_explicit_errors() {
        let program = program();
        assert!(resolve(&program, VTableId::new(1), 0).unwrap_err().contains("invalid vtable"));
        assert!(resolve(&program, VTableId::new(0), 3).unwrap_err().contains("no slot"));
        assert!(find_slot(&program, VTableId::new(0), "missing").unwrap_err().contains("no method"));
    }
}
