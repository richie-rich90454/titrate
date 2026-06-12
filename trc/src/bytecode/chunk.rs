// Titrate Alpha 0.2 – crafted by richie-rich90454, 2026

// ---------------------------------------------------------------------------
// Chunk – a compiled unit of bytecode
// ---------------------------------------------------------------------------

use super::opcodes::OpCode;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<u64>,
    pub strings: Vec<String>,
    pub source_lines: Vec<u32>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            strings: Vec::new(),
            source_lines: Vec::new(),
        }
    }

    // -- Writing helpers -----------------------------------------------------

    pub fn write_opcode(&mut self, op: OpCode, line: u32) {
        self.code.push(op as u8);
        self.source_lines.push(line);
    }

    pub fn write_u8(&mut self, value: u8, line: u32) {
        self.code.push(value);
        self.source_lines.push(line);
    }

    pub fn write_u16(&mut self, value: u16, line: u32) {
        let bytes = value.to_be_bytes();
        self.code.push(bytes[0]);
        self.source_lines.push(line);
        self.code.push(bytes[1]);
        self.source_lines.push(line);
    }

    pub fn write_i16(&mut self, value: i16, line: u32) {
        let bytes = value.to_be_bytes();
        self.code.push(bytes[0]);
        self.source_lines.push(line);
        self.code.push(bytes[1]);
        self.source_lines.push(line);
    }

    // -- String interning ----------------------------------------------------

    pub fn add_string(&mut self, s: &str) -> u16 {
        if let Some(idx) = self.strings.iter().position(|existing| existing == s) {
            return idx as u16;
        }
        let idx = self.strings.len() as u16;
        self.strings.push(s.to_owned());
        idx
    }

    // -- Reading helpers -----------------------------------------------------

    pub fn read_u8(&self, offset: usize) -> u8 {
        self.code[offset]
    }

    pub fn read_u16(&self, offset: usize) -> u16 {
        let hi = self.code[offset] as u16;
        let lo = self.code[offset + 1] as u16;
        (hi << 8) | lo
    }

    pub fn read_i16(&self, offset: usize) -> i16 {
        let hi = self.code[offset];
        let lo = self.code[offset + 1];
        i16::from_be_bytes([hi, lo])
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_chunk_write_opcode() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        chunk.write_opcode(OpCode::POP, 2);
        assert_eq!(chunk.code.len(), 2);
        assert_eq!(chunk.code[0], OpCode::PUSH_NULL as u8);
        assert_eq!(chunk.code[1], OpCode::POP as u8);
        assert_eq!(chunk.source_lines[0], 1);
        assert_eq!(chunk.source_lines[1], 2);
    }

    #[test]
    fn test_chunk_constants() {
        let mut chunk = Chunk::new();

        // PUSH_I32 takes a 4-byte immediate operand
        chunk.write_opcode(OpCode::PUSH_I32, 10);
        chunk.write_u8(0x00, 10);
        chunk.write_u8(0x00, 10);
        chunk.write_u8(0x01, 10);
        chunk.write_u8(0x2C, 10); // 300

        assert_eq!(chunk.code.len(), 5);
        assert_eq!(chunk.read_u8(0), OpCode::PUSH_I32 as u8);
        // Reconstruct the i32 from the operand bytes
        let val = (chunk.read_u8(1) as u32) << 24
            | (chunk.read_u8(2) as u32) << 16
            | (chunk.read_u8(3) as u32) << 8
            | chunk.read_u8(4) as u32;
        assert_eq!(val, 300u32);
    }

    #[test]
    fn test_string_interning() {
        let mut chunk = Chunk::new();
        let idx1 = chunk.add_string("hello");
        let idx2 = chunk.add_string("world");
        let idx3 = chunk.add_string("hello"); // should reuse

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0);
        assert_eq!(chunk.strings.len(), 2);
        assert_eq!(chunk.strings[0], "hello");
        assert_eq!(chunk.strings[1], "world");
    }

    #[test]
    fn test_read_write_u16() {
        let mut chunk = Chunk::new();
        chunk.write_u16(0xABCD, 1);
        assert_eq!(chunk.read_u16(0), 0xABCD);

        // Round-trip through write_i16 / read_i16
        let mut chunk2 = Chunk::new();
        chunk2.write_i16(-1234, 1);
        assert_eq!(chunk2.read_i16(0), -1234);
    }

    #[test]
    fn test_all_opcodes_have_distinct_values() {
        let mut seen: HashMap<u8, String> = HashMap::new();
        let mut conflicts = Vec::new();

        for variant in OpCode::variants() {
            let val = *variant as u8;
            let name = format!("{:?}", variant);
            if let Some(prev) = seen.insert(val, name.clone()) {
                conflicts.push(format!(
                    "OpCode value {} used by both {} and {}",
                    val, prev, name
                ));
            }
        }

        assert!(
            conflicts.is_empty(),
            "Duplicate opcode discriminants found: {:?}",
            conflicts
        );
    }

    // Helper – iterate all defined OpCode variants via discriminant range.
    impl OpCode {
        fn variants() -> &'static [OpCode] {
            // Since OpCode is repr(u8) and contiguous 0..=127, we can build
            // the list at test time via try_from.
            (0u8..=127)
                .map(|v| OpCode::try_from(v).expect("valid opcode"))
                .collect::<Vec<_>>()
                .leak()
        }
    }
}
