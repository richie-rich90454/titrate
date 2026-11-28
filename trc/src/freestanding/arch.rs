//! Core-only volatile, MMIO, and inline-assembly primitives.
//!
//! Device registers must never be reached with an ordinary load or store: the
//! compiler may cache or reorder those. Keeping the primitives here gives the
//! rest of the runtime one place where volatile access is guaranteed.

/// Read `T` from `address` exactly once, with volatile semantics.
///
/// # Safety
///
/// `address` must be aligned for `T`, mapped, and readable as `T` for the whole
/// call. Device registers may change between reads.
#[inline(always)]
pub unsafe fn read_volatile<T>(address: usize) -> T {
    // SAFETY: guaranteed by the caller.
    unsafe { core::ptr::read_volatile(address as *const T) }
}

/// Write `value` to `address` exactly once, with volatile semantics.
///
/// # Safety
///
/// `address` must be aligned for `T`, mapped, and writable as `T` for the whole
/// call. Some devices require a specific width; use the matching `T`.
#[inline(always)]
pub unsafe fn write_volatile<T>(address: usize, value: T) {
    // SAFETY: guaranteed by the caller.
    unsafe { core::ptr::write_volatile(address as *mut T, value) }
}

/// Read the register at `base + offset`, returning `None` on address overflow.
///
/// # Safety
///
/// The computed address must be a mapped, aligned, readable `T` register.
#[inline(always)]
pub unsafe fn mmio_read<T>(base: usize, offset: usize) -> Option<T> {
    let address = base.checked_add(offset)?;
    Some(read_volatile(address))
}

/// Write the register at `base + offset`, returning `None` on address overflow.
///
/// # Safety
///
/// The computed address must be a mapped, aligned, writable `T` register.
#[inline(always)]
pub unsafe fn mmio_write<T>(base: usize, offset: usize, value: T) -> Option<()> {
    let address = base.checked_add(offset)?;
    write_volatile(address, value);
    Some(())
}

/// Read one byte from an x86 I/O port.
///
/// # Safety
///
/// `port` must be a port the hardware allows the CPU to read in the current
/// privilege level.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub unsafe fn port_in_u8(port: u16) -> u8 {
    let value: u8;
    // SAFETY: `dx` and `al` are fixed by the instruction; no memory is touched.
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nostack, preserves_flags)
        );
    }
    value
}

/// Read one byte from an x86 I/O port.
///
/// # Safety
///
/// Port I/O does not exist on this architecture.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
pub unsafe fn port_in_u8(_port: u16) -> u8 {
    unreachable!("x86 port I/O is not available on this architecture")
}

/// Write one byte to an x86 I/O port.
///
/// # Safety
///
/// `port` must be a port the hardware allows the CPU to write in the current
/// privilege level.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub unsafe fn port_out_u8(port: u16, value: u8) {
    // SAFETY: `dx` and `al` are fixed by the instruction; no memory is touched.
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nostack, preserves_flags)
        );
    }
}

/// Write one byte to an x86 I/O port.
///
/// # Safety
///
/// Port I/O does not exist on this architecture.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
pub unsafe fn port_out_u8(_port: u16, _value: u8) {
    unreachable!("x86 port I/O is not available on this architecture")
}

/// Disable maskable interrupts.
///
/// # Safety
///
/// The caller must re-enable interrupts before returning to the kernel's
/// normal execution path.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub unsafe fn interrupts_disable() {
    // SAFETY: `cli` changes IF only, which is the caller's explicit request.
    unsafe { core::arch::asm!("cli", options(nostack, preserves_flags)) };
}

/// Disable maskable interrupts.
///
/// # Safety
///
/// Interrupts cannot be disabled on this architecture.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
pub unsafe fn interrupts_disable() {
    unreachable!("x86 interrupt flags are not available on this architecture")
}

/// Enable maskable interrupts.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn interrupts_enable() {
    // SAFETY: `sti` changes IF only; interrupt handlers must already be ready.
    unsafe { core::arch::asm!("sti", options(nostack, preserves_flags)) };
}

/// Enable maskable interrupts.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
pub fn interrupts_enable() {
    core::hint::spin_loop();
}

/// Hint to the CPU that the caller is in a spin-wait loop.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn pause() {
    // SAFETY: `pause` only reduces spin-loop power usage.
    unsafe { core::arch::asm!("pause", options(nostack, preserves_flags)) };
}

/// Hint to the CPU that the caller is in a spin-wait loop.
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
pub fn pause() {
    core::hint::spin_loop();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volatile_round_trip_reads_the_last_write() {
        let mut cell = 0u32;
        let address = &mut cell as *mut u32 as usize;
        unsafe { write_volatile(address, 0xA5A5_A5A5u32) };
        assert_eq!(unsafe { read_volatile::<u32>(address) }, 0xA5A5_A5A5);
        assert_eq!(cell, 0xA5A5_A5A5);
    }

    #[test]
    fn mmio_helpers_reject_overflowing_addresses() {
        let mut cell = 7u8;
        let base = &mut cell as *mut u8 as usize;
        assert_eq!(unsafe { mmio_read::<u8>(base, 0) }, Some(7));
        assert!(unsafe { mmio_read::<u8>(usize::MAX, 1) }.is_none());
        assert_eq!(unsafe { mmio_write::<u8>(base, 0, 9) }, Some(()));
        assert!(unsafe { mmio_write::<u8>(usize::MAX, 1, 0u8) }.is_none());
        assert_eq!(cell, 9);
    }

    #[test]
    fn pause_is_safe_to_call_on_a_host() {
        pause();
    }
}
