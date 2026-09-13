// Titrate Alpha 0.2 - bytecode virtual machine: tests
// Precision in every step - richie-rich90454, 2026

    use crate::bytecode::chunk::Chunk;
    use crate::bytecode::frame::{FunctionDef};
    use crate::bytecode::vm::Vm;

    /// Helper: create a minimal VM with a single function (main) containing
    /// the given bytecode and an empty string table.
    fn vm_with_chunk(chunk: Chunk) -> Vm {
        let mut vm = Vm::new();
        vm.add_function(FunctionDef {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_method: false,
            is_constructor: false,
            local_count: 0,
        param_types: Vec::new(),
        });
        vm
    }

mod core;
mod closures_data;
mod json_time;
mod units_net;
mod strings;
mod natives_ext;
