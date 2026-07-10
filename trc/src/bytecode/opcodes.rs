// Titrate Alpha 0.2 – crafted by richie-rich90454, 2026

// ---------------------------------------------------------------------------
// Cast target types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CastTarget {
    Byte   = 0,
    Short  = 1,
    Int    = 2,
    Long   = 3,
    Vast   = 4,
    Uvast  = 5,
    Float  = 6,
    Double = 7,
    Half   = 8,
    Quad   = 9,
    Char   = 10,
    String = 11,
    Bool   = 12,
}

// ---------------------------------------------------------------------------
// Type tags (used by TYPE_CHECK and CAST)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TypeTag {
    I8       = 0,
    I16      = 1,
    I32      = 2,
    I64      = 3,
    I128     = 4,
    U128     = 5,
    F32      = 6,
    F64      = 7,
    Bool     = 8,
    Char     = 9,
    String   = 10,
    Null     = 11,
    Void     = 12,
    Class    = 13,
    Enum     = 14,
    Array    = 15,
    Ref      = 16,
    Owned    = 17,
    Result   = 18,
    Function = 19,
}

// ---------------------------------------------------------------------------
// Opcodes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum OpCode {
    // -- Constants -----------------------------------------------------------
    PUSH_I8    = 0,
    PUSH_I16   = 1,
    PUSH_I32   = 2,
    PUSH_I64   = 3,
    PUSH_F32   = 4,
    PUSH_F64   = 5,
    PUSH_BOOL  = 6,
    PUSH_CHAR  = 7,
    PUSH_STRING = 8,
    PUSH_NULL  = 9,
    PUSH_VOID  = 10,

    // -- Stack ---------------------------------------------------------------
    POP  = 11,
    DUP  = 12,
    SWAP = 13,

    // -- Arithmetic ----------------------------------------------------------
    ADD_I32 = 14,
    ADD_I64 = 15,
    ADD_F32 = 16,
    ADD_F64 = 17,
    SUB_I32 = 18,
    SUB_I64 = 19,
    SUB_F32 = 20,
    SUB_F64 = 21,
    MUL_I32 = 22,
    MUL_I64 = 23,
    MUL_F32 = 24,
    MUL_F64 = 25,
    DIV_I32 = 26,
    DIV_I64 = 27,
    DIV_F32 = 28,
    DIV_F64 = 29,
    MOD_I32 = 30,
    MOD_I64 = 31,
    MOD_F32 = 32,
    MOD_F64 = 33,
    NEG_I32 = 34,
    NEG_I64 = 35,
    NEG_F32 = 36,
    NEG_F64 = 37,

    // -- Bitwise -------------------------------------------------------------
    BITAND_I32 = 38,
    BITAND_I64 = 39,
    BITOR_I32  = 40,
    BITOR_I64  = 41,
    BITXOR_I32 = 42,
    BITXOR_I64 = 43,
    SHL_I32    = 44,
    SHL_I64    = 45,
    SHR_I32    = 46,
    SHR_I64    = 47,
    BITNOT_I32 = 48,
    BITNOT_I64 = 49,

    // -- Comparison ----------------------------------------------------------
    EQ_I32    = 50,
    EQ_I64    = 51,
    EQ_F32    = 52,
    EQ_F64    = 53,
    EQ_BOOL   = 54,
    EQ_CHAR   = 55,
    EQ_STRING = 56,
    NE_I32    = 57,
    NE_I64    = 58,
    NE_F32    = 59,
    NE_F64    = 60,
    LT_I32    = 61,
    LT_I64    = 62,
    LT_F32    = 63,
    LT_F64    = 64,
    LE_I32    = 65,
    LE_I64    = 66,
    LE_F32    = 67,
    LE_F64    = 68,
    GT_I32    = 69,
    GT_I64    = 70,
    GT_F32    = 71,
    GT_F64    = 72,
    GE_I32    = 73,
    GE_I64    = 74,
    GE_F32    = 75,
    GE_F64    = 76,

    // -- Logic ---------------------------------------------------------------
    AND = 77,
    OR  = 78,
    NOT = 79,

    // -- String --------------------------------------------------------------
    STR_CONCAT      = 80,
    STR_CONCAT_RIGHT = 81,
    STR_CONCAT_LEFT  = 82,

    // -- Control flow --------------------------------------------------------
    JMP            = 83,
    JMP_IF_FALSE   = 84,
    JMP_IF_TRUE    = 85,
    CALL           = 86,
    RET            = 87,
    CALL_NATIVE    = 88,

    // -- Variables -----------------------------------------------------------
    LOAD_LOCAL   = 89,
    STORE_LOCAL  = 90,
    LOAD_UPVALUE  = 91,
    STORE_UPVALUE = 92,

    // -- Objects -------------------------------------------------------------
    NEW              = 93,
    INVOKE_VIRTUAL   = 94,
    GET_FIELD        = 95,
    SET_FIELD        = 96,

    // -- Arrays --------------------------------------------------------------
    ARRAY_NEW  = 97,
    ARRAY_GET  = 98,
    ARRAY_SET  = 99,
    ARRAY_LEN  = 100,

    // -- Ownership -----------------------------------------------------------
    BOX_VALUE     = 101,
    UNBOX_VALUE   = 102,
    REGION_ALLOC  = 103,
    REF_IMMUTABLE = 104,
    REF_MUTABLE   = 105,
    DEREF         = 106,

    // -- Enum ----------------------------------------------------------------
    ENUM_NEW = 107,

    // -- Result --------------------------------------------------------------
    RESULT_OK          = 108,
    RESULT_ERR         = 109,
    UNWRAP_OR_PROPAGATE = 110,

    // -- Cast ----------------------------------------------------------------
    CAST = 111,

    // -- For iteration -------------------------------------------------------
    ITER_NEXT = 112,

    // -- Switch / Pattern matching -------------------------------------------
    MATCH_ENUM = 113,
    MATCH_OK   = 114,
    MATCH_ERR  = 115,

    // -- Built-in static calls -----------------------------------------------
    STATIC_CALL = 116,

    // -- Type narrowing ------------------------------------------------------
    TYPE_CHECK = 117,

    // -- Super constructor call -----------------------------------------------
    CALL_SUPER = 118,

    // -- Region deallocation --------------------------------------------------
    FREE_REGION = 119,

    // -- Closures -------------------------------------------------------------
    CLOSURE_NEW = 120,
    GET_UPVALUE = 121,
    SET_UPVALUE = 122,

    // -- Operator overloading -------------------------------------------------
    INVOKE_OPERATOR = 123,

    // -- Tuples ---------------------------------------------------------------
    TUPLE_NEW = 124,
    TUPLE_GET = 125,

    // -- Closure capture ------------------------------------------------------
    CLOSURE_NEW_CAPTURED = 126,
    CLOSURE_CAPTURE      = 127,

    // -- Unsigned right shift -------------------------------------------------
    USHR_I32 = 128, // unsigned (logical) right shift, i32
    USHR_I64 = 129, // unsigned (logical) right shift, i64

    // -- Module-level globals -------------------------------------------------
    LOAD_GLOBAL  = 130, // u16 global index
    STORE_GLOBAL = 131, // u16 global index

    // -- Closure call ----------------------------------------------------------
    CALL_CLOSURE = 132, // u8 arg count — pops closure from stack, then args

    // -- Exception handling ---------------------------------------------------
    THROW         = 133, // pops a value and throws it as an exception
    PUSH_HANDLER  = 134, // u16 catch IP (absolute offset in current chunk)
    POP_HANDLER   = 135, // removes the top exception handler

    // -- Type check (class) --------------------------------------------------
    INSTANCE_OF = 136, // u16 class name string index — pops value, pushes bool
}

impl OpCode {
    /// Returns the number of operand bytes that follow this opcode.
    /// Operands are laid out sequentially after the opcode byte.
    pub fn operand_size(&self) -> usize {
        match self {
            // Constants: immediate operand sizes
            Self::PUSH_I8    => 1,
            Self::PUSH_I16   => 2,
            Self::PUSH_I32   => 4,
            Self::PUSH_I64   => 8,
            Self::PUSH_F32   => 4,
            Self::PUSH_F64   => 8,
            Self::PUSH_BOOL  => 1,
            Self::PUSH_CHAR  => 4, // Unicode scalar value
            Self::PUSH_STRING => 2, // u16 index into string table

            // Control flow
            Self::JMP           => 2, // i16 offset
            Self::JMP_IF_FALSE  => 2,
            Self::JMP_IF_TRUE   => 2,
            Self::CALL          => 3, // u16 function index + u8 arg count
            Self::CALL_NATIVE   => 3, // u16 native fn index + u8 arg count

            // Variables
            Self::LOAD_LOCAL    => 1, // u8 slot
            Self::STORE_LOCAL   => 1,
            Self::LOAD_UPVALUE  => 1,
            Self::STORE_UPVALUE => 1,
            Self::LOAD_GLOBAL   => 2, // u16 global index
            Self::STORE_GLOBAL  => 2, // u16 global index

            // Objects
            Self::NEW             => 3, // u16 class index + u8 arg count
            Self::INVOKE_VIRTUAL  => 3, // u16 method name + u8 arg count
            Self::GET_FIELD       => 2, // u16 field name
            Self::SET_FIELD       => 2,

            // Arrays
            Self::ARRAY_NEW => 2, // u16 size

            // Enum
            Self::ENUM_NEW => 5, // u16 enum name + u16 variant name + u8 field count

            // Cast
            Self::CAST => 1, // u8 target type

            // Iteration
            Self::ITER_NEXT => 2, // i16 jump offset if exhausted

            // Pattern matching
            Self::MATCH_ENUM => 4, // u16 variant name + i16 jump offset
            Self::MATCH_OK   => 2, // i16 jump offset
            Self::MATCH_ERR  => 2, // i16 jump offset

            // Static call
            Self::STATIC_CALL => 5, // u16 class name + u16 method name + u8 arg count

            // Type narrowing
            Self::TYPE_CHECK => 1, // u8 type tag

            // Super constructor call
            Self::CALL_SUPER => 3, // u16 function index + u8 arg count

            // Closures
            Self::CLOSURE_NEW => 3, // u16 function index + u8 upvalue count
            Self::GET_UPVALUE => 1, // u8 upvalue index
            Self::SET_UPVALUE => 1, // u8 upvalue index

            // Operator overloading
            Self::INVOKE_OPERATOR => 3, // u16 method name + u8 arg count

            // Tuples
            Self::TUPLE_NEW => 2, // u16 element count
            Self::TUPLE_GET => 1, // u8 element index

            // Closure capture
            Self::CLOSURE_NEW_CAPTURED => 3, // u16 function index + u8 captured count
            Self::CLOSURE_CAPTURE      => 1, // u8 local slot index

            // Closure call
            Self::CALL_CLOSURE => 1, // u8 arg count

            // Exception handling
            Self::THROW        => 0,
            Self::PUSH_HANDLER => 2, // u16 catch IP
            Self::POP_HANDLER  => 0,

            // Type check (class)
            Self::INSTANCE_OF => 2, // u16 class name string index

            // Everything else: no operands
            _ => 0,
        }
    }

    /// Total instruction length in bytes (opcode + operands).
    pub fn instruction_size(&self) -> usize {
        1 + self.operand_size()
    }
}

impl From<OpCode> for u8 {
    fn from(op: OpCode) -> u8 {
        op as u8
    }
}

impl TryFrom<u8> for OpCode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // Validate that the value falls within the defined opcode range.
        match value {
            // SAFETY: `OpCode` is declared `#[repr(u8)]` and has exactly 136 unit
            // variants with explicit discriminants 0 (PUSH_I8) through 135
            // (POP_HANDLER). The match arm guard above constrains `value` to the
            // inclusive range 0..=135, so every possible byte value reaching the
            // transmute corresponds to a valid enum discriminant. Because the
            // representation is `u8` and the enum carries no payload, the bit
            // pattern of `value` is identical to the in-memory representation of
            // the corresponding `OpCode` variant; the transmute therefore
            // produces a valid discriminant and does not invoke undefined
            // behavior. Values outside 0..=135 are rejected via `Err`.
            0..=136 => Ok(unsafe { std::mem::transmute::<u8, OpCode>(value) }),
            _ => Err(value),
        }
    }
}


