use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::chunk::Chunk;
use super::value::Value;

/// One activation on the call stack.
///
/// Each time a function is invoked, a new `Frame` is pushed. When the
/// function returns, the frame is popped and the caller's state is restored.
pub struct Frame {
    /// Index into the VM's function table.
    pub function_index: u16,
    /// Instruction pointer – offset into the chunk's code.
    pub ip: usize,
    /// Base pointer – index into the value stack where this frame's locals start.
    pub base: usize,
    /// Captured upvalues for closures. None for regular functions.
    pub upvalues: Option<Vec<Rc<RefCell<Value>>>>,
}

impl Frame {
    pub fn new(function_index: u16, base: usize) -> Self {
        Self {
            function_index,
            ip: 0,
            base,
            upvalues: None,
        }
    }

    pub fn new_with_upvalues(function_index: u16, base: usize, upvalues: Vec<Rc<RefCell<Value>>>) -> Self {
        Self {
            function_index,
            ip: 0,
            base,
            upvalues: Some(upvalues),
        }
    }
}

/// A compiled function ready for the VM to execute.
pub struct FunctionDef {
    pub name: String,
    /// Number of positional parameters.
    pub arity: usize,
    /// The bytecode for this function.
    pub chunk: Chunk,
    /// Whether this function is a class method.
    pub is_method: bool,
    /// Whether this function is a constructor (`init`).
    pub is_constructor: bool,
    /// Total number of local variable slots (params + locals).
    /// The VM pre-allocates this many stack slots when the function is called.
    pub local_count: usize,
}

/// A compiled class definition.
pub struct ClassDef {
    pub name: String,
    /// Index into the class table for the parent class, if any.
    pub parent: Option<u16>,
    pub fields: Vec<FieldDef>,
    /// Method name → function index.
    pub methods: HashMap<String, u16>,
    /// Function index of the constructor, if one exists.
    pub constructor: Option<u16>,
    /// Field name together with the chunk that computes its initial value.
    pub field_inits: Vec<(String, Chunk)>,
}

/// Describes a single field declared in a class body.
pub struct FieldDef {
    pub name: String,
    /// Whether the field has an initialiser expression.
    pub has_init: bool,
}

/// A compiled enum definition.
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<VariantDef>,
}

/// One variant of an enum.
pub struct VariantDef {
    pub name: String,
    /// Number of positional fields carried by this variant.
    pub field_count: usize,
}

/// An exception handler registered by a try/catch block.
/// When a THROW occurs, the VM unwinds to the nearest handler.
#[derive(Clone)]
pub struct ExceptionHandler {
    /// Function index of the frame that owns this handler.
    pub function_index: u16,
    /// IP within that function's chunk to jump to (catch block start).
    pub catch_ip: usize,
    /// Stack depth at the time PUSH_HANDLER was executed.
    /// The stack is truncated to this depth before pushing the exception value.
    pub stack_depth: usize,
    /// Frame depth at the time PUSH_HANDLER was executed.
    /// Frames above this depth are popped during unwinding.
    pub frame_depth: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_new() {
        let frame = Frame::new(3, 12);
        assert_eq!(frame.function_index, 3);
        assert_eq!(frame.ip, 0);
        assert_eq!(frame.base, 12);
    }

    #[test]
    fn test_function_def() {
        let def = FunctionDef {
            name: "greet".into(),
            arity: 2,
            chunk: Chunk::new(),
            is_method: false,
            is_constructor: false,
            local_count: 4,
        };
        assert_eq!(def.name, "greet");
        assert_eq!(def.arity, 2);
        assert!(!def.is_method);
        assert!(!def.is_constructor);
        assert_eq!(def.local_count, 4);
    }

    #[test]
    fn test_class_def_with_inheritance() {
        let mut methods = HashMap::new();
        methods.insert("speak".into(), 0u16);
        methods.insert("init".into(), 1u16);

        let class = ClassDef {
            name: "Dog".into(),
            parent: Some(0), // inherits from class at index 0 (e.g. "Animal")
            fields: vec![
                FieldDef {
                    name: "name".into(),
                    has_init: true,
                },
                FieldDef {
                    name: "age".into(),
                    has_init: false,
                },
            ],
            methods,
            constructor: Some(1),
            field_inits: vec![("name".into(), Chunk::new())],
        };

        assert_eq!(class.name, "Dog");
        assert_eq!(class.parent, Some(0));
        assert_eq!(class.fields.len(), 2);
        assert!(class.fields[0].has_init);
        assert!(!class.fields[1].has_init);
        assert_eq!(class.methods.len(), 2);
        assert_eq!(class.methods["speak"], 0);
        assert_eq!(class.constructor, Some(1));
        assert_eq!(class.field_inits.len(), 1);
    }
}
