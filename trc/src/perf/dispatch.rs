//! Allocation-free native dispatch stubs.
//!
//! The current VM looks natives up by `String` and hands each call a freshly
//! collected `Vec<Value>`. This table uses static names and borrows the
//! caller's argument slice instead, so lookup and invocation perform no heap
//! allocation. It is a drop-in shape for a future `Vm` native table, not a
//! replacement for the capability checks the VM still performs before
//! dispatch.

use std::fmt;

use crate::bytecode::value::{NativeFn, Value};

/// Fixed capacity keeps the table inline and makes lookup cache-friendly.
pub const DISPATCH_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeDispatchError {
    DuplicateName,
    TableFull,
    UnknownName,
}

impl fmt::Display for NativeDispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            NativeDispatchError::DuplicateName => "native name already registered",
            NativeDispatchError::TableFull => "native dispatch table is full",
            NativeDispatchError::UnknownName => "native name must not be empty",
        };
        f.write_str(text)
    }
}

impl std::error::Error for NativeDispatchError {}

type Slot = (&'static str, Option<NativeFn>);

pub struct NativeDispatch {
    slots: [Slot; DISPATCH_CAPACITY],
    len: usize,
}

impl Default for NativeDispatch {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeDispatch {
    pub const fn new() -> Self {
        Self {
            slots: [("", None); DISPATCH_CAPACITY],
            len: 0,
        }
    }

    /// Register a native under a static name; returns its stable index.
    pub fn register(
        &mut self,
        name: &'static str,
        native: NativeFn,
    ) -> Result<u16, NativeDispatchError> {
        if name.is_empty() {
            return Err(NativeDispatchError::UnknownName);
        }
        if self.slots[..self.len]
            .iter()
            .any(|(existing, _)| *existing == name)
        {
            return Err(NativeDispatchError::DuplicateName);
        }
        if self.len == DISPATCH_CAPACITY {
            return Err(NativeDispatchError::TableFull);
        }
        let idx = self.len;
        self.slots[idx] = (name, Some(native));
        self.len += 1;
        Ok(idx as u16)
    }

    /// Linear scan over static names: no `String` construction on lookup.
    pub fn lookup(&self, name: &str) -> Option<u16> {
        self.slots[..self.len]
            .iter()
            .position(|(existing, _)| *existing == name)
            .map(|idx| idx as u16)
    }

    /// Call by index with the caller's argument slice, without collecting it.
    pub fn invoke(&self, idx: u16, args: &[Value]) -> Result<Value, String> {
        let native = self
            .slots
            .get(idx as usize)
            .and_then(|(_, native)| native.as_ref());
        match native {
            Some(native) => (*native)(args),
            None => Err(format!("unknown native index {}", idx)),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native_add(args: &[Value]) -> Result<Value, String> {
        let lhs = args.first().and_then(Value::to_i64).ok_or("missing lhs")?;
        let rhs = args.get(1).and_then(Value::to_i64).ok_or("missing rhs")?;
        Ok(Value::Long(lhs + rhs))
    }

    fn native_zero(_args: &[Value]) -> Result<Value, String> {
        Ok(Value::Long(0))
    }

    #[test]
    fn register_lookup_and_invoke() {
        let mut table = NativeDispatch::new();
        assert!(table.is_empty());
        let idx = table.register("add", native_add).expect("register add");
        assert_eq!(table.len(), 1);
        assert_eq!(table.lookup("add"), Some(idx));
        assert_eq!(table.lookup("missing"), None);
        let result = table
            .invoke(idx, &[Value::Long(2), Value::Long(3)])
            .expect("invoke add");
        assert_eq!(result.to_i64(), Some(5));
    }

    #[test]
    fn duplicate_and_empty_names_are_rejected() {
        let mut table = NativeDispatch::new();
        table.register("add", native_add).expect("register add");
        assert_eq!(
            table.register("add", native_add),
            Err(NativeDispatchError::DuplicateName)
        );
        assert_eq!(
            table.register("", native_add),
            Err(NativeDispatchError::UnknownName)
        );
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn unknown_index_is_an_error_not_a_panic() {
        let table = NativeDispatch::new();
        assert!(table.invoke(9999, &[]).is_err());
        assert!(table.invoke(0, &[]).is_err());
    }

    #[test]
    fn table_reports_full_capacity() {
        let mut table = NativeDispatch::new();
        for idx in 0..DISPATCH_CAPACITY {
            table
                .register(static_name(idx), native_zero)
                .expect("fill slot");
        }
        assert_eq!(table.len(), DISPATCH_CAPACITY);
        assert_eq!(
            table.register("overflow", native_zero),
            Err(NativeDispatchError::TableFull)
        );
    }

    /// Test-only supply of distinct static names for the capacity check.
    fn static_name(idx: usize) -> &'static str {
        const NAMES: [&str; DISPATCH_CAPACITY] = [
            "n0", "n1", "n2", "n3", "n4", "n5", "n6", "n7", "n8", "n9", "n10", "n11", "n12",
            "n13", "n14", "n15", "n16", "n17", "n18", "n19", "n20", "n21", "n22", "n23", "n24",
            "n25", "n26", "n27", "n28", "n29", "n30", "n31", "n32", "n33", "n34", "n35", "n36",
            "n37", "n38", "n39", "n40", "n41", "n42", "n43", "n44", "n45", "n46", "n47", "n48",
            "n49", "n50", "n51", "n52", "n53", "n54", "n55", "n56", "n57", "n58", "n59", "n60",
            "n61", "n62", "n63",
        ];
        NAMES[idx]
    }
}
