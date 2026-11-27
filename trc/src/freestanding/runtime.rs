//! Core-only freestanding runtime.
//!
//! This file deliberately uses `core` only. To build the image, copy it into a
//! `titrate_rt` crate root with `#![no_std]` and `panic = "abort"`, then link
//! with `-nostdlib -nostartfiles -nodefaultlibs -T x86_64-none.ld`. As part of
//! the hosted `trc` library the hardware pieces are cfg-gated, so hosted
//! behavior and panic handling are untouched.
//!
//! The stub reports its state over COM1 and halts. It does not load a kernel,
//! set up paging, or implement UEFI boot; those capabilities are reported as
//! unsupported instead of silently succeeding.
//!
//! Everything below this point is `core`-only and can be compiled unchanged as
//! a module of a `#![no_std]` `titrate_rt` crate. The host-side target
//! validation stays in the parent module.

use core::fmt;

#[cfg(target_os = "none")]
use core::fmt::Write as _;

/// Failure from a serial write. Carries a reason instead of a bare `()` so a
/// caller can report why output was dropped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SerialError(&'static str);

impl SerialError {
    /// Stable explanation of the failure.
    pub const fn reason(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for SerialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Byte sink used by [`Serial`]. Any device that cannot write must return an
/// error rather than silently discarding bytes.
pub trait SerialWriter {
    /// Write one byte, or report why it could not be written.
    fn write_byte(&mut self, byte: u8) -> Result<(), SerialError>;
}

/// Writer for builds with no serial device configured.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSerial;

impl SerialWriter for NoSerial {
    fn write_byte(&mut self, _byte: u8) -> Result<(), SerialError> {
        Err(SerialError("no serial device configured for this target"))
    }
}

/// Allocator-free formatting over a [`SerialWriter`].
#[derive(Clone, Copy, Debug)]
pub struct Serial<W> {
    writer: W,
}

impl<W: SerialWriter> Serial<W> {
    /// Wrap a byte sink.
    pub const fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Access the underlying sink.
    pub fn writer(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Write a string's bytes verbatim.
    pub fn write_str(&mut self, text: &str) -> Result<(), SerialError> {
        for &byte in text.as_bytes() {
            self.writer.write_byte(byte)?;
        }
        Ok(())
    }

    /// Write a string followed by CRLF.
    pub fn write_line(&mut self, text: &str) -> Result<(), SerialError> {
        self.write_str(text)?;
        self.write_str("\r\n")
    }

    /// Write an unsigned integer in radix 2..=16, lowercase for `a`..`f`.
    pub fn write_u64(&mut self, value: u64, radix: u8) -> Result<(), SerialError> {
        if !(2..=16).contains(&radix) {
            return Err(SerialError("radix must be between 2 and 16"));
        }
        let base = radix as u64;
        let mut digits = [0u8; 64];
        let mut pos = digits.len();
        let mut remaining = value;
        if remaining == 0 {
            pos -= 1;
            digits[pos] = b'0';
        } else {
            while remaining != 0 {
                let digit = (remaining % base) as u8;
                pos -= 1;
                digits[pos] = if digit < 10 {
                    b'0' + digit
                } else {
                    b'a' + (digit - 10)
                };
                remaining /= base;
            }
        }
        let text = core::str::from_utf8(&digits[pos..])
            .map_err(|_| SerialError("formatted digit is not UTF-8"))?;
        self.write_str(text)
    }

    /// Write a signed decimal integer, including `i64::MIN`.
    pub fn write_i64(&mut self, value: i64) -> Result<(), SerialError> {
        if value < 0 {
            self.write_str("-")?;
            self.write_u64(value.unsigned_abs(), 10)
        } else {
            self.write_u64(value as u64, 10)
        }
    }

    /// Write an unsigned integer as lowercase hexadecimal.
    pub fn write_hex(&mut self, value: u64) -> Result<(), SerialError> {
        self.write_u64(value, 16)
    }

    /// Write a boolean as `true` or `false`.
    pub fn write_bool(&mut self, value: bool) -> Result<(), SerialError> {
        self.write_str(if value { "true" } else { "false" })
    }

    /// Write a runtime value using its canonical textual form.
    pub fn write_value(&mut self, value: Value) -> Result<(), SerialError> {
        match value {
            Value::Bool(flag) => self.write_bool(flag),
            Value::Int(number) => self.write_i64(number),
            Value::UInt(number) => self.write_u64(number, 10),
        }
    }
}

impl<W: SerialWriter> fmt::Write for Serial<W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        Serial::write_str(self, text).map_err(|_| fmt::Error)
    }
}

/// Fixed-capacity byte sink, used by tests and by any caller that cannot
/// allocate.
#[derive(Debug)]
pub struct BufferWriter<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> BufferWriter<N> {
    /// Create an empty buffer.
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    /// Bytes written so far, decoded as UTF-8.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    /// Number of bytes written.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether no bytes have been written.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> Default for BufferWriter<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> SerialWriter for BufferWriter<N> {
    fn write_byte(&mut self, byte: u8) -> Result<(), SerialError> {
        if self.len == N {
            return Err(SerialError("serial buffer full"));
        }
        self.bytes[self.len] = byte;
        self.len += 1;
        Ok(())
    }
}

/// The only value shapes the runtime can represent without an allocator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Value {
    /// Boolean value.
    Bool(bool),
    /// Signed integer.
    Int(i64),
    /// Unsigned integer.
    UInt(u64),
}

/// Explicit failure from a value operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueOpError {
    /// Checked arithmetic overflowed.
    Overflow(&'static str),
    /// The operation is not defined for these operands.
    Unsupported(&'static str),
}

impl ValueOpError {
    /// Stable explanation of the failure.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Overflow(operation) => operation,
            Self::Unsupported(operation) => operation,
        }
    }
}

impl fmt::Display for ValueOpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow(operation) => core::write!(f, "{} overflowed", operation),
            Self::Unsupported(operation) => {
                core::write!(f, "{} is not supported freestanding", operation)
            }
        }
    }
}

impl Value {
    /// Checked addition. Mixed numeric widths and booleans are rejected.
    pub fn checked_add_value(self, rhs: Self) -> Result<Self, ValueOpError> {
        match (self, rhs) {
            (Self::Int(a), Self::Int(b)) => a
                .checked_add(b)
                .map(Self::Int)
                .ok_or(ValueOpError::Overflow("int addition")),
            (Self::UInt(a), Self::UInt(b)) => a
                .checked_add(b)
                .map(Self::UInt)
                .ok_or(ValueOpError::Overflow("uint addition")),
            _ => Err(ValueOpError::Unsupported("adding these values")),
        }
    }

    /// Checked subtraction. Mixed numeric widths and booleans are rejected.
    pub fn checked_sub_value(self, rhs: Self) -> Result<Self, ValueOpError> {
        match (self, rhs) {
            (Self::Int(a), Self::Int(b)) => a
                .checked_sub(b)
                .map(Self::Int)
                .ok_or(ValueOpError::Overflow("int subtraction")),
            (Self::UInt(a), Self::UInt(b)) => a
                .checked_sub(b)
                .map(Self::UInt)
                .ok_or(ValueOpError::Overflow("uint subtraction")),
            _ => Err(ValueOpError::Unsupported("subtracting these values")),
        }
    }

    /// Checked multiplication. Mixed numeric widths and booleans are rejected.
    pub fn checked_mul_value(self, rhs: Self) -> Result<Self, ValueOpError> {
        match (self, rhs) {
            (Self::Int(a), Self::Int(b)) => a
                .checked_mul(b)
                .map(Self::Int)
                .ok_or(ValueOpError::Overflow("int multiplication")),
            (Self::UInt(a), Self::UInt(b)) => a
                .checked_mul(b)
                .map(Self::UInt)
                .ok_or(ValueOpError::Overflow("uint multiplication")),
            _ => Err(ValueOpError::Unsupported("multiplying these values")),
        }
    }

    /// Checked negation. Unsigned values and booleans are rejected.
    pub fn checked_neg_value(self) -> Result<Self, ValueOpError> {
        match self {
            Self::Int(a) => a
                .checked_neg()
                .map(Self::Int)
                .ok_or(ValueOpError::Overflow("int negation")),
            _ => Err(ValueOpError::Unsupported("negating this value")),
        }
    }

    /// Truthiness used by freestanding conditionals.
    pub fn is_truthy(self) -> bool {
        match self {
            Self::Bool(flag) => flag,
            Self::Int(number) => number != 0,
            Self::UInt(number) => number != 0,
        }
    }
}

/// Magic value the bootloader places in `eax` for a multiboot2 image.
pub const MULTIBOOT2_MAGIC: u32 = 0xE852_50D6;
/// Architecture field for the i386/x86-64 multiboot2 header.
pub const MULTIBOOT2_ARCHITECTURE_I386: u32 = 0;
/// Size of the magic, architecture, length, checksum, and end tag in bytes.
pub const MULTIBOOT2_HEADER_LEN: u32 = 24;
/// Size of the terminating multiboot2 tag.
pub const MULTIBOOT2_END_TAG_SIZE: u32 = 8;

/// Checksum that makes the header sum to zero as required by multiboot2.
pub const fn multiboot2_checksum() -> u32 {
    MULTIBOOT2_MAGIC
        .wrapping_add(MULTIBOOT2_ARCHITECTURE_I386)
        .wrapping_add(MULTIBOOT2_HEADER_LEN)
        .wrapping_neg()
}

/// Minimal multiboot2 header: the mandatory fields plus the end tag.
#[repr(C, align(8))]
pub struct Multiboot2Header {
    /// Must be [`MULTIBOOT2_MAGIC`].
    pub magic: u32,
    /// Boot architecture; zero for i386 and x86-64.
    pub architecture: u32,
    /// Total header size, including tags.
    pub header_length: u32,
    /// Negative sum of the preceding fields and the length.
    pub checksum: u32,
    /// End tag type, zero.
    pub end_tag_type: u16,
    /// End tag flags, zero.
    pub end_tag_flags: u16,
    /// End tag size, eight bytes.
    pub end_tag_size: u32,
}

impl Multiboot2Header {
    /// Build the header constant.
    pub const fn new() -> Self {
        Self {
            magic: MULTIBOOT2_MAGIC,
            architecture: MULTIBOOT2_ARCHITECTURE_I386,
            header_length: MULTIBOOT2_HEADER_LEN,
            checksum: multiboot2_checksum(),
            end_tag_type: 0,
            end_tag_flags: 0,
            end_tag_size: MULTIBOOT2_END_TAG_SIZE,
        }
    }
}

impl Default for Multiboot2Header {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "none")]
#[used]
#[link_section = ".multiboot"]
static MULTIBOOT2_HEADER: Multiboot2Header = Multiboot2Header::new();

/// Data port of the first serial port.
pub const COM1_DATA_PORT: u16 = 0x03F8;
/// Line status port of the first serial port.
pub const COM1_STATUS_PORT: u16 = 0x03FD;
/// Line status bit that indicates the transmit holding register is empty.
pub const COM1_TRANSMIT_EMPTY: u8 = 0x20;

/// Serial writer that emits bytes through COM1.
#[derive(Clone, Copy, Debug, Default)]
pub struct Com1;

#[cfg(target_arch = "x86_64")]
impl SerialWriter for Com1 {
    fn write_byte(&mut self, byte: u8) -> Result<(), SerialError> {
        unsafe {
            let status: u8;
            core::arch::asm!(
                "in al, dx",
                in("dx") COM1_STATUS_PORT,
                out("al") status,
                options(nostack)
            );
            if status & COM1_TRANSMIT_EMPTY == 0 {
                return Err(SerialError("COM1 transmit holding register is busy"));
            }
            core::arch::asm!(
                "out dx, al",
                in("dx") COM1_DATA_PORT,
                in("al") byte,
                options(nostack)
            );
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "x86_64"))]
impl SerialWriter for Com1 {
    fn write_byte(&mut self, _byte: u8) -> Result<(), SerialError> {
        Err(SerialError("COM1 port I/O is only available on x86_64"))
    }
}

/// Halt the CPU. Never returns.
pub fn halt() -> ! {
    #[cfg(target_arch = "x86_64")]
    loop {
        unsafe { core::arch::asm!("hlt", options(nostack)) };
    }

    #[cfg(not(target_arch = "x86_64"))]
    loop {
        core::hint::spin_loop();
    }
}

/// Entry called by `_start` after it validates the multiboot2 magic.
///
/// `info` is the physical address of the multiboot2 information structure.
/// The stub does not parse it, because no kernel image is loaded yet.
#[no_mangle]
pub extern "C" fn trt_entry(magic: u32, info: *const u8) -> ! {
    let mut serial = Serial::new(Com1);
    let _ = info;
    if magic != MULTIBOOT2_MAGIC {
        let _ = serial.write_str("titrate: invalid multiboot2 magic 0x");
        let _ = serial.write_hex(magic as u64);
        let _ = serial.write_str("\r\n");
        halt();
    }
    let _ = serial.write_line("titrate: multiboot2 stub reached; no kernel image is loaded");
    halt()
}

/// UEFI status code for an unimplemented operation.
pub const EFI_UNSUPPORTED: u64 = 0x8000_0000_0000_0003;

/// UEFI entry point. UEFI boot is not implemented, so this returns a defined
/// unsupported status instead of pretending to initialize the runtime.
///
/// The stub ignores both arguments, so the SysV/MS ABI difference in argument
/// registers does not affect it.
#[no_mangle]
pub extern "C" fn efi_main(_image_handle: usize, _system_table: *const u8) -> u64 {
    EFI_UNSUPPORTED
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    let mut serial = Serial::new(Com1);
    let _ = serial.write_str("titrate: panic: ");
    if let Some(message) = info.message() {
        let _ = serial.write_fmt(*message);
    }
    let _ = serial.write_str("\r\n");
    halt()
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
core::arch::global_asm!(
    r#"
.intel_syntax noprefix
.global _start
.type _start, @function
_start:
    cmp eax, 0xe85250d6
    jne 2f
    mov esi, eax
    mov rdi, rbx
    and rsp, -16
    call trt_entry
2:
    cli
    hlt
    jmp 2b
.size _start, . - _start
.att_syntax prefix
"#
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiboot2_header_has_expected_layout() {
        let header = Multiboot2Header::new();
        assert_eq!(header.magic, MULTIBOOT2_MAGIC);
        assert_eq!(header.architecture, MULTIBOOT2_ARCHITECTURE_I386);
        assert_eq!(header.header_length, MULTIBOOT2_HEADER_LEN);
        assert_eq!(header.end_tag_type, 0);
        assert_eq!(header.end_tag_flags, 0);
        assert_eq!(header.end_tag_size, MULTIBOOT2_END_TAG_SIZE);
        assert_eq!(core::mem::size_of::<Multiboot2Header>(), 24);
    }

    #[test]
    fn multiboot2_header_sums_to_zero() {
        let sum = MULTIBOOT2_MAGIC
            .wrapping_add(MULTIBOOT2_ARCHITECTURE_I386)
            .wrapping_add(MULTIBOOT2_HEADER_LEN)
            .wrapping_add(multiboot2_checksum());
        assert_eq!(sum, 0);
    }

    #[test]
    fn uefi_stub_reports_unsupported() {
        assert_eq!(
            efi_main(0, core::ptr::null()),
            EFI_UNSUPPORTED,
            "UEFI boot must not claim success"
        );
    }
}
