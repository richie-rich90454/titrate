// Chunk emission helpers and builtin class creation

use crate::ast;
use super::super::frame::ClassDef;
use super::super::chunk::Chunk;
use super::super::opcodes::OpCode;
use super::Compiler;

impl Compiler {
    // -----------------------------------------------------------------------
    // Chunk access helpers
    // -----------------------------------------------------------------------

    pub(super) fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.functions[self.current_function].chunk
    }

    pub(super) fn emit_opcode(&mut self, op: OpCode, line: u32) {
        self.current_chunk().write_opcode(op, line);
    }

    pub(super) fn emit_u8(&mut self, value: u8, line: u32) {
        self.current_chunk().write_u8(value, line);
    }

    pub(super) fn emit_u16(&mut self, value: u16, line: u32) {
        self.current_chunk().write_u16(value, line);
    }

    pub(super) fn emit_i16(&mut self, value: i16, line: u32) {
        self.current_chunk().write_i16(value, line);
    }

    pub(super) fn current_ip(&mut self) -> usize {
        self.current_chunk().code.len()
    }

    pub(super) fn intern_string(&mut self, s: &str) -> u16 {
        self.current_chunk().add_string(s)
    }

    pub(super) fn patch_i16_at(&mut self, offset: usize, value: i16) {
        let bytes = value.to_be_bytes();
        let chunk = self.current_chunk();
        chunk.code[offset] = bytes[0];
        chunk.code[offset + 1] = bytes[1];
    }

    // -----------------------------------------------------------------------
    // Builtin class creation
    // -----------------------------------------------------------------------

    /// Get or create a built-in pseudo-class (ArrayList, HashMap, etc.)
    /// Uses monomorphization naming: ArrayList<int> → ArrayList__int
    pub(super) fn get_or_create_builtin_class(&mut self, name: &str, type_args: &[ast::Type]) -> u16 {
        let mangled = Self::mangle_name(name, type_args);

        if let Some(&idx) = self.class_map.get(&mangled) {
            return idx;
        }
        let idx = self.classes.len() as u16;
        let class_def = ClassDef {
            name: mangled.clone(),
            parent: None,
            fields: Vec::new(),
            methods: std::collections::HashMap::new(),
            constructor: None,
            field_inits: Vec::new(),
        };
        self.classes.push(class_def);
        self.class_map.insert(mangled.clone(), idx);

        // Also cache in mono_cache.
        self.mono_cache.insert(mangled, idx);

        idx
    }
}
