//! Interrupt descriptor table setup for the freestanding image.
//!
//! The table is assembled at runtime from empty gates and installed exactly
//! once with `lidt` before the kernel unmasks interrupts. CPU exception
//! vectors are reserved: this module only installs gates for device vectors.

use core::fmt;

/// Number of descriptors in the x86-64 IDT.
pub const IDT_ENTRIES: usize = 256;
/// First vector the kernel may hand to a device.
pub const FIRST_DEVICE_VECTOR: u16 = 32;
/// Descriptor is present in memory.
pub const GATE_PRESENT: u8 = 1 << 7;
/// 64-bit interrupt gate; entry enters with IF cleared.
pub const GATE_TYPE_INTERRUPT: u8 = 0x0E;
/// 64-bit trap gate; entry preserves IF.
pub const GATE_TYPE_TRAP: u8 = 0x0F;
/// Privilege level 0, the only level the stub uses.
pub const GATE_DPL0: u8 = 0x00;
/// Privilege level 3, for user-callable gates.
pub const GATE_DPL3: u8 = 0x60;
/// Mask for the three stack-switch IST selectors in a gate.
const IST_MASK: u8 = 0x07;

/// A single 16-byte x86-64 interrupt gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    flags: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    /// An unpresent gate, used to fill the table before devices are installed.
    pub const fn empty() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            flags: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Build a present-capable gate for `handler`.
    pub const fn new(handler: u64, selector: u16, ist: u8, flags: u8) -> Self {
        Self {
            offset_low: handler as u16,
            selector,
            ist: ist & IST_MASK,
            flags,
            offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }

    /// Reassemble the 64-bit handler address from the split offset fields.
    pub fn handler(self) -> u64 {
        self.offset_low as u64
            | ((self.offset_mid as u64) << 16)
            | ((self.offset_high as u64) << 32)
    }

    /// Code segment selector used on entry.
    pub fn selector(self) -> u16 {
        self.selector
    }

    /// IST index, in `0..=7`.
    pub fn ist(self) -> u8 {
        self.ist
    }

    /// Type, privilege, and present bits.
    pub fn flags(self) -> u8 {
        self.flags
    }

    /// Whether the CPU will accept this gate.
    pub fn is_present(self) -> bool {
        self.flags & GATE_PRESENT != 0
    }
}

/// Why a gate could not be installed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdtError {
    vector: u16,
    reason: &'static str,
}

impl IdtError {
    fn new(vector: u16, reason: &'static str) -> Self {
        Self { vector, reason }
    }

    /// Vector the caller tried to install.
    pub fn vector(&self) -> u16 {
        self.vector
    }

    /// Stable explanation of the failure.
    pub fn reason(&self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for IdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IDT vector {}: {}", self.vector, self.reason)
    }
}

/// Descriptor table with 256 empty gates.
#[derive(Clone, Debug)]
pub struct Idt {
    entries: [IdtEntry; IDT_ENTRIES],
}

impl Idt {
    /// Build a table whose gates are all unpresent.
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::empty(); IDT_ENTRIES],
        }
    }

    /// Install a gate for a device vector.
    pub fn set_gate(
        &mut self,
        vector: u16,
        handler: u64,
        selector: u16,
        ist: u8,
        flags: u8,
    ) -> Result<(), IdtError> {
        if vector as usize >= IDT_ENTRIES {
            return Err(IdtError::new(vector, "vector is outside the IDT"));
        }
        if vector < FIRST_DEVICE_VECTOR {
            return Err(IdtError::new(vector, "vector is reserved for CPU exceptions"));
        }
        if flags & GATE_PRESENT == 0 {
            return Err(IdtError::new(vector, "gate is not present"));
        }
        if ist > IST_MASK {
            return Err(IdtError::new(vector, "IST index must be in 0..=7"));
        }
        self.entries[vector as usize] = IdtEntry::new(handler, selector, ist, flags);
        Ok(())
    }

    /// Copy a gate by vector, or `None` if the vector is outside the table.
    pub fn entry(&self, vector: u16) -> Option<IdtEntry> {
        self.entries.get(vector as usize).copied()
    }

    /// Address of the first descriptor, used to build the IDT pointer.
    pub fn as_ptr(&self) -> *const IdtEntry {
        self.entries.as_ptr()
    }

    fn pointer(&self) -> IdtPointer {
        IdtPointer {
            limit: (IDT_ENTRIES * core::mem::size_of::<IdtEntry>() - 1) as u16,
            base: self.as_ptr() as u64,
        }
    }

    /// Install this table as the CPU's IDT.
    ///
    /// # Safety
    ///
    /// Every present gate must point to a valid handler, and the selector must
    /// describe a valid code segment. `lidt` is privileged: call this only from
    /// the freestanding kernel running at privilege level 0.
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    pub unsafe fn load(&self) {
        let pointer = self.pointer();
        // SAFETY: `pointer` lives for the whole asm block and describes a
        // table that outlives the call through `&self`.
        unsafe {
            core::arch::asm!("lidt [{}]", in(reg) &pointer, options(nostack, preserves_flags));
        }
    }

    /// Install this table as the CPU's IDT.
    ///
    /// # Safety
    ///
    /// `lidt` is privileged and only emitted for the freestanding image.
    #[cfg(not(all(target_arch = "x86_64", target_os = "none")))]
    pub unsafe fn load(&self) {
        let pointer = self.pointer();
        let _ = (pointer.limit, pointer.base);
        unreachable!("lidt is only available in the freestanding x86_64 image");
    }
}

impl Default for Idt {
    fn default() -> Self {
        Self::new()
    }
}

/// The 10-byte operand `lidt` expects: a 16-bit limit followed by a 64-bit
/// base. Padding would make the descriptor limit wrong, so the layout is packed.
#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_and_pointer_layouts_are_exact() {
        assert_eq!(core::mem::size_of::<IdtEntry>(), 16);
        assert_eq!(core::mem::size_of::<Idt>(), 4096);
        assert_eq!(core::mem::size_of::<IdtPointer>(), 10);
    }

    #[test]
    fn handler_offset_survives_the_split_fields() {
        let handler = 0xFFFF_8000_1234_5678u64;
        let gate = IdtEntry::new(handler, 0x08, 2, GATE_PRESENT | GATE_TYPE_INTERRUPT);
        assert_eq!(gate.handler(), handler);
        assert_eq!(gate.selector(), 0x08);
        assert_eq!(gate.ist(), 2);
        assert!(gate.is_present());
    }

    #[test]
    fn ist_is_masked_to_three_bits() {
        let gate = IdtEntry::new(0x1000, 0x08, 0xFF, GATE_PRESENT);
        assert_eq!(gate.ist(), 0x07);
    }

    #[test]
    fn fresh_table_has_no_present_gates() {
        let idt = Idt::new();
        assert!(idt.entry(0).unwrap().handler() == 0);
        assert!(!idt.entry(32).unwrap().is_present());
        assert!(idt.entry(IDT_ENTRIES as u16).is_none());
    }

    #[test]
    fn device_gate_is_installed_at_its_vector() {
        let mut idt = Idt::new();
        idt.set_gate(
            0x30,
            0x1234,
            0x08,
            1,
            GATE_PRESENT | GATE_TYPE_INTERRUPT | GATE_DPL0,
        )
        .expect("device vector is installable");
        let gate = idt.entry(0x30).unwrap();
        assert_eq!(gate.handler(), 0x1234);
        assert!(gate.is_present());
        assert_eq!(gate.flags(), GATE_PRESENT | GATE_TYPE_INTERRUPT | GATE_DPL0);
        assert!(!idt.entry(0x31).unwrap().is_present());
    }

    #[test]
    fn invalid_gates_are_rejected_with_the_vector() {
        let mut idt = Idt::new();
        let cases = [
            (CPU_RESERVED_VECTOR, "reserved", GATE_PRESENT, 0),
            (IDT_ENTRIES as u16, "outside", GATE_PRESENT, 0),
            (0x30, "not present", GATE_TYPE_TRAP, 0),
            (0x30, "IST", GATE_PRESENT, 8),
        ];
        for (vector, reason, flags, ist) in cases {
            let err = idt
                .set_gate(vector, 0x1000, 0x08, ist, flags)
                .expect_err("gate must be rejected");
            assert_eq!(err.vector(), vector);
            assert!(err.reason().contains(reason));
            assert!(!err.to_string().is_empty());
        }
    }

    #[test]
    fn idt_pointer_describes_the_whole_table() {
        let idt = Idt::new();
        let pointer = idt.pointer();
        let limit = pointer.limit;
        let base = pointer.base;
        assert_eq!(limit, 4095);
        assert_eq!(base, idt.as_ptr() as u64);
    }

    const CPU_RESERVED_VECTOR: u16 = FIRST_DEVICE_VECTOR - 1;
}
