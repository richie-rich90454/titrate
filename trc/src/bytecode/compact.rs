// Compact value representation for a future TIR-backed VM.
// Additive only: the live VM keeps using bytecode::value::Value.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::bytecode::value::Value;

/// Why a conversion or a field/slot lookup failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompactError {
    /// The source `Value` carries state (an OS handle or a shared mutable
    /// cell) that cannot be flattened into an immutable compact payload.
    NotRepresentable(&'static str),
    UnknownField(String),
    UnknownSlot(u16),
}

impl fmt::Display for CompactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompactError::NotRepresentable(what) => {
                write!(f, "value not representable as CompactValue: {what}")
            }
            CompactError::UnknownField(name) => write!(f, "unknown field: {name}"),
            CompactError::UnknownSlot(slot) => write!(f, "unknown slot: {slot}"),
        }
    }
}

impl std::error::Error for CompactError {}

/// Tag word for a [`CompactValue`]. The enum discriminant is the actual tag;
/// this type exposes it so a future VM can switch on a small integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompactTag {
    Void,
    Null,
    Moved,
    Bool,
    Byte,
    Short,
    Int,
    Long,
    Vast,
    Uvast,
    Float,
    Double,
    Half,
    Quad,
    Char,
    Function,
    NativeFn,
    Ref,
    RawPtr,
    Heap,
}

/// A tagged pointer union.
///
/// Small scalars are carried inline in the enum payload. Every bulk value is
/// routed through the single [`CompactValue::Heap`] variant, so the heap
/// representation is exactly one `Rc<HeapValue>` pointer plus a tag. This
/// keeps the slot small enough for the TIR-backed VM without changing the
/// existing [`Value`].
#[derive(Clone, Debug, PartialEq)]
pub enum CompactValue {
    Void,
    Null,
    Moved,
    Bool(bool),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Vast(i128),
    Uvast(u128),
    Float(f32),
    Double(f64),
    Half(f32),
    Quad(f64),
    Char(char),
    Function(u16),
    NativeFn(u16),
    Ref(usize),
    RawPtr(usize),
    Heap(Rc<HeapValue>),
}

/// Payload behind the `Heap` tag.
#[derive(Clone, Debug, PartialEq)]
pub enum HeapValue {
    String(String),
    Array(Vec<CompactValue>),
    Tuple(Vec<CompactValue>),
    Instance(Instance),
    EnumInstance(EnumInstance),
    ResultOk(CompactValue),
    ResultErr(CompactValue),
    Owned(CompactValue),
    EnumVariant {
        enum_name: String,
        variant: String,
        field_count: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumInstance {
    pub enum_name: String,
    pub variant: String,
    pub fields: Vec<CompactValue>,
}

impl CompactValue {
    /// Convert a live `Value`, rejecting anything that has no flat compact
    /// representation.
    pub fn from_value(value: &Value) -> Result<Self, CompactError> {
        let compact = match value {
            Value::Void => CompactValue::Void,
            Value::Null => CompactValue::Null,
            Value::Moved => CompactValue::Moved,
            Value::Bool(v) => CompactValue::Bool(*v),
            Value::Byte(v) => CompactValue::Byte(*v),
            Value::Short(v) => CompactValue::Short(*v),
            Value::Int(v) => CompactValue::Int(*v),
            Value::Long(v) => CompactValue::Long(*v),
            Value::Vast(v) => CompactValue::Vast(*v),
            Value::Uvast(v) => CompactValue::Uvast(*v),
            Value::Float(v) => CompactValue::Float(*v),
            Value::Double(v) => CompactValue::Double(*v),
            Value::Half(v) => CompactValue::Half(*v),
            Value::Quad(v) => CompactValue::Quad(*v),
            Value::Char(v) => CompactValue::Char(*v),
            Value::Function(v) => CompactValue::Function(*v),
            Value::NativeFn(v) => CompactValue::NativeFn(*v),
            Value::Ref(v) => CompactValue::Ref(*v),
            Value::RawPtr(v) => CompactValue::RawPtr(*v),
            Value::String(s) => {
                CompactValue::Heap(Rc::new(HeapValue::String((**s).clone())))
            }
            Value::Array { elements } => {
                let values = elements
                    .iter()
                    .map(CompactValue::from_value)
                    .collect::<Result<Vec<_>, _>>()?;
                CompactValue::Heap(Rc::new(HeapValue::Array(values)))
            }
            Value::Tuple { elements } => {
                let values = elements
                    .iter()
                    .map(CompactValue::from_value)
                    .collect::<Result<Vec<_>, _>>()?;
                CompactValue::Heap(Rc::new(HeapValue::Tuple(values)))
            }
            Value::Owned(inner) => CompactValue::Heap(Rc::new(HeapValue::Owned(
                CompactValue::from_value(inner.as_ref())?,
            ))),
            Value::ResultOk(inner) => CompactValue::Heap(Rc::new(HeapValue::ResultOk(
                CompactValue::from_value(inner.as_ref())?,
            ))),
            Value::ResultErr(inner) => CompactValue::Heap(Rc::new(HeapValue::ResultErr(
                CompactValue::from_value(inner.as_ref())?,
            ))),
            Value::EnumInstance {
                enum_name,
                variant,
                fields,
            } => {
                let values = fields
                    .iter()
                    .map(CompactValue::from_value)
                    .collect::<Result<Vec<_>, _>>()?;
                CompactValue::Heap(Rc::new(HeapValue::EnumInstance(EnumInstance {
                    enum_name: enum_name.clone(),
                    variant: variant.clone(),
                    fields: values,
                })))
            }
            Value::EnumVariant {
                enum_name,
                variant,
                field_count,
            } => CompactValue::Heap(Rc::new(HeapValue::EnumVariant {
                enum_name: enum_name.clone(),
                variant: variant.clone(),
                field_count: *field_count,
            })),
            Value::ClassInstance {
                class_name,
                fields,
                vtable,
            } => {
                let field_names: Vec<String> =
                    fields.borrow().keys().cloned().collect();
                let mut method_names: Vec<&String> = vtable.keys().collect();
                method_names.sort();
                let mut entries = Vec::new();
                for name in method_names {
                    for &method in &vtable[name] {
                        entries.push(VTableEntry {
                            name: name.clone(),
                            method,
                        });
                    }
                }
                let layout = Rc::new(ClassLayout::new(
                    class_name.clone(),
                    field_names,
                    Rc::new(VTable::new(entries)),
                ));
                let mut instance = Instance::new(layout);
                for (name, field) in fields.borrow().iter() {
                    instance.set_field(name, CompactValue::from_value(field)?)?;
                }
                CompactValue::Heap(Rc::new(HeapValue::Instance(instance)))
            }
            Value::Cell(_) => {
                return Err(CompactError::NotRepresentable(
                    "Cell (shared mutable capture)",
                ))
            }
            Value::Closure { .. } => {
                return Err(CompactError::NotRepresentable("Closure"))
            }
            Value::FileHandle(_) => {
                return Err(CompactError::NotRepresentable("FileHandle"))
            }
            Value::Socket(_) => return Err(CompactError::NotRepresentable("Socket")),
            Value::Listener(_) => {
                return Err(CompactError::NotRepresentable("Listener"))
            }
        };
        Ok(compact)
    }

    /// Rebuild a live `Value`. Provided for round-trip testing and for a
    /// future VM that needs to hand a compact value back to legacy code.
    pub fn to_value(&self) -> Result<Value, CompactError> {
        let value = match self {
            CompactValue::Void => Value::Void,
            CompactValue::Null => Value::Null,
            CompactValue::Moved => Value::Moved,
            CompactValue::Bool(v) => Value::Bool(*v),
            CompactValue::Byte(v) => Value::Byte(*v),
            CompactValue::Short(v) => Value::Short(*v),
            CompactValue::Int(v) => Value::Int(*v),
            CompactValue::Long(v) => Value::Long(*v),
            CompactValue::Vast(v) => Value::Vast(*v),
            CompactValue::Uvast(v) => Value::Uvast(*v),
            CompactValue::Float(v) => Value::Float(*v),
            CompactValue::Double(v) => Value::Double(*v),
            CompactValue::Half(v) => Value::Half(*v),
            CompactValue::Quad(v) => Value::Quad(*v),
            CompactValue::Char(v) => Value::Char(*v),
            CompactValue::Function(v) => Value::Function(*v),
            CompactValue::NativeFn(v) => Value::NativeFn(*v),
            CompactValue::Ref(v) => Value::Ref(*v),
            CompactValue::RawPtr(v) => Value::RawPtr(*v),
            CompactValue::Heap(heap) => match heap.as_ref() {
                HeapValue::String(s) => Value::String(Rc::new(s.clone())),
                HeapValue::Array(values) => Value::Array {
                    elements: values
                        .iter()
                        .map(CompactValue::to_value)
                        .collect::<Result<Vec<_>, _>>()?,
                },
                HeapValue::Tuple(values) => Value::Tuple {
                    elements: values
                        .iter()
                        .map(CompactValue::to_value)
                        .collect::<Result<Vec<_>, _>>()?,
                },
                HeapValue::Owned(inner) => {
                    Value::Owned(Box::new(inner.to_value()?))
                }
                HeapValue::ResultOk(inner) => {
                    Value::ResultOk(Box::new(inner.to_value()?))
                }
                HeapValue::ResultErr(inner) => {
                    Value::ResultErr(Box::new(inner.to_value()?))
                }
                HeapValue::EnumInstance(EnumInstance {
                    enum_name,
                    variant,
                    fields,
                }) => Value::EnumInstance {
                    enum_name: enum_name.clone(),
                    variant: variant.clone(),
                    fields: fields
                        .iter()
                        .map(CompactValue::to_value)
                        .collect::<Result<Vec<_>, _>>()?,
                },
                HeapValue::EnumVariant {
                    enum_name,
                    variant,
                    field_count,
                } => Value::EnumVariant {
                    enum_name: enum_name.clone(),
                    variant: variant.clone(),
                    field_count: *field_count,
                },
                HeapValue::Instance(instance) => {
                    let mut fields = HashMap::new();
                    for (slot, name) in instance.layout.field_names.iter().enumerate() {
                        fields.insert(name.clone(), instance.slots[slot].to_value()?);
                    }
                    let mut vtable: HashMap<String, Vec<u16>> = HashMap::new();
                    for entry in instance.layout.vtable.entries() {
                        vtable
                            .entry(entry.name.clone())
                            .or_default()
                            .push(entry.method);
                    }
                    Value::ClassInstance {
                        class_name: instance.layout.class_name.clone(),
                        fields: Rc::new(std::cell::RefCell::new(fields)),
                        vtable,
                    }
                }
            },
        };
        Ok(value)
    }

    pub fn tag(&self) -> CompactTag {
        match self {
            CompactValue::Void => CompactTag::Void,
            CompactValue::Null => CompactTag::Null,
            CompactValue::Moved => CompactTag::Moved,
            CompactValue::Bool(_) => CompactTag::Bool,
            CompactValue::Byte(_) => CompactTag::Byte,
            CompactValue::Short(_) => CompactTag::Short,
            CompactValue::Int(_) => CompactTag::Int,
            CompactValue::Long(_) => CompactTag::Long,
            CompactValue::Vast(_) => CompactTag::Vast,
            CompactValue::Uvast(_) => CompactTag::Uvast,
            CompactValue::Float(_) => CompactTag::Float,
            CompactValue::Double(_) => CompactTag::Double,
            CompactValue::Half(_) => CompactTag::Half,
            CompactValue::Quad(_) => CompactTag::Quad,
            CompactValue::Char(_) => CompactTag::Char,
            CompactValue::Function(_) => CompactTag::Function,
            CompactValue::NativeFn(_) => CompactTag::NativeFn,
            CompactValue::Ref(_) => CompactTag::Ref,
            CompactValue::RawPtr(_) => CompactTag::RawPtr,
            CompactValue::Heap(_) => CompactTag::Heap,
        }
    }

    pub fn is_heap(&self) -> bool {
        matches!(self, CompactValue::Heap(_))
    }

    pub fn is_inline(&self) -> bool {
        !self.is_heap()
    }
}

impl TryFrom<&Value> for CompactValue {
    type Error = CompactError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        CompactValue::from_value(value)
    }
}

/// One method in a shared vtable. `slot` is the index into
/// [`VTable::entries`]; `method` is the VM function index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VTableEntry {
    pub name: String,
    pub method: u16,
}

/// A shared, immutable dispatch table indexed by `u16` slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VTable {
    entries: Vec<VTableEntry>,
    by_name: HashMap<String, u16>,
}

impl VTable {
    pub fn new(entries: Vec<VTableEntry>) -> Self {
        let mut by_name = HashMap::new();
        for (slot, entry) in entries.iter().enumerate() {
            by_name.entry(entry.name.clone()).or_insert(slot as u16);
        }
        Self { entries, by_name }
    }

    pub fn lookup(&self, slot: u16) -> Option<&VTableEntry> {
        self.entries.get(slot as usize)
    }

    pub fn slot_of(&self, name: &str) -> Option<u16> {
        self.by_name.get(name).copied()
    }

    pub fn entries(&self) -> &[VTableEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Sorted field names define stable slots for every instance of a class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassLayout {
    pub class_name: String,
    field_names: Vec<String>,
    field_slots: HashMap<String, u16>,
    vtable: Rc<VTable>,
}

impl ClassLayout {
    pub fn new(
        class_name: impl Into<String>,
        mut field_names: Vec<String>,
        vtable: Rc<VTable>,
    ) -> Self {
        field_names.sort();
        field_names.dedup();
        let field_slots = field_names
            .iter()
            .enumerate()
            .map(|(slot, name)| (name.clone(), slot as u16))
            .collect();
        Self {
            class_name: class_name.into(),
            field_names,
            field_slots,
            vtable,
        }
    }

    pub fn slot(&self, name: &str) -> Option<u16> {
        self.field_slots.get(name).copied()
    }

    pub fn field_names(&self) -> &[String] {
        &self.field_names
    }

    pub fn field_count(&self) -> usize {
        self.field_names.len()
    }

    pub fn vtable(&self) -> &Rc<VTable> {
        &self.vtable
    }
}

/// Interns class layouts so instances of the same class share one `Rc`.
#[derive(Debug, Default)]
pub struct LayoutRegistry {
    layouts: Vec<Rc<ClassLayout>>,
}

impl LayoutRegistry {
    pub fn intern(
        &mut self,
        class_name: &str,
        field_names: &[String],
        vtable: Rc<VTable>,
    ) -> Rc<ClassLayout> {
        let mut sorted = field_names.to_vec();
        sorted.sort();
        sorted.dedup();
        if let Some(found) = self.layouts.iter().find(|layout| {
            layout.class_name == class_name
                && layout.field_names.as_slice() == sorted.as_slice()
                && layout.vtable.as_ref() == vtable.as_ref()
        }) {
            return Rc::clone(found);
        }
        let layout = Rc::new(ClassLayout::new(class_name, sorted, vtable));
        self.layouts.push(Rc::clone(&layout));
        layout
    }
}

/// An instance is a layout pointer plus one compact value per field slot.
#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    layout: Rc<ClassLayout>,
    slots: Vec<CompactValue>,
}

impl Instance {
    pub fn new(layout: Rc<ClassLayout>) -> Self {
        let slots = vec![CompactValue::Null; layout.field_count()];
        Self { layout, slots }
    }

    pub fn get_field(&self, name: &str) -> Result<&CompactValue, CompactError> {
        let slot = self
            .layout
            .slot(name)
            .ok_or_else(|| CompactError::UnknownField(name.to_string()))?;
        self.get_slot(slot)
    }

    pub fn set_field(
        &mut self,
        name: &str,
        value: CompactValue,
    ) -> Result<(), CompactError> {
        let slot = self
            .layout
            .slot(name)
            .ok_or_else(|| CompactError::UnknownField(name.to_string()))?;
        self.slots[slot as usize] = value;
        Ok(())
    }

    pub fn get_slot(&self, slot: u16) -> Result<&CompactValue, CompactError> {
        self.slots
            .get(slot as usize)
            .ok_or(CompactError::UnknownSlot(slot))
    }

    pub fn method(&self, slot: u16) -> Option<&VTableEntry> {
        self.layout.vtable().lookup(slot)
    }

    pub fn layout(&self) -> &Rc<ClassLayout> {
        &self.layout
    }

    pub fn slots(&self) -> &[CompactValue] {
        &self.slots
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn inline_primitives_round_trip() {
        let values = vec![
            Value::Void,
            Value::Null,
            Value::Moved,
            Value::Bool(true),
            Value::Byte(-3),
            Value::Short(300),
            Value::Int(42_000),
            Value::Long(-9_000_000_000),
            Value::Vast(i128::MIN),
            Value::Uvast(u128::MAX),
            Value::Float(1.5),
            Value::Double(-2.25),
            Value::Half(0.5),
            Value::Quad(3.75),
            Value::Char('z'),
            Value::Function(17),
            Value::NativeFn(9),
            Value::Ref(4),
            Value::RawPtr(8),
        ];

        for value in values {
            let compact = CompactValue::from_value(&value).unwrap();
            assert!(compact.is_inline(), "{value:?} should be inline");
            assert!(!compact.is_heap());
            assert_ne!(compact.tag(), CompactTag::Heap);
            assert_eq!(compact.to_value().unwrap(), value);
        }

        assert_eq!(CompactValue::Int(1).tag(), CompactTag::Int);
        assert_eq!(CompactValue::Uvast(1).tag(), CompactTag::Uvast);
        assert_eq!(CompactValue::Char('a').tag(), CompactTag::Char);
    }

    #[test]
    fn heap_variants_round_trip() {
        let cases: Vec<(Value, &str)> = vec![
            (
                Value::String(Rc::new("hello".to_string())),
                "hello",
            ),
            (
                Value::Array {
                    elements: vec![Value::Int(1), Value::Bool(false)],
                },
                "[1, false]",
            ),
            (
                Value::Tuple {
                    elements: vec![Value::Int(2), Value::Char('c')],
                },
                "(2, c)",
            ),
            (
                Value::ResultOk(Box::new(Value::Int(11))),
                "Ok(11)",
            ),
            (
                Value::ResultErr(Box::new(Value::String(Rc::new("bad".to_string())))),
                "Err(bad)",
            ),
            (
                Value::EnumInstance {
                    enum_name: "Color".to_string(),
                    variant: "RGB".to_string(),
                    fields: vec![Value::Int(1), Value::Int(2)],
                },
                "RGB(1, 2)",
            ),
            (
                Value::EnumVariant {
                    enum_name: "Option".to_string(),
                    variant: "Some".to_string(),
                    field_count: 1,
                },
                "<variant Some>",
            ),
            (Value::Owned(Box::new(Value::Int(6))), "6"),
        ];

        for (value, display) in cases {
            let compact = CompactValue::from_value(&value).unwrap();
            assert!(compact.is_heap(), "{value:?} should be heap-backed");
            assert_eq!(compact.tag(), CompactTag::Heap);
            let back = compact.to_value().unwrap();
            assert_eq!(back.display_string(), display);
        }

        // Those with a structural PartialEq compare directly too.
        let array = Value::Array {
            elements: vec![Value::Int(1), Value::Int(2)],
        };
        assert_eq!(CompactValue::from_value(&array).unwrap().to_value().unwrap(), array);
    }

    #[test]
    fn instance_uses_sorted_field_slots_and_shared_layout() {
        let vtable = Rc::new(VTable::new(vec![]));
        let mut registry = LayoutRegistry::default();
        let names = vec!["z".to_string(), "a".to_string(), "m".to_string()];

        let first = registry.intern("Thing", &names, Rc::clone(&vtable));
        let second = registry.intern("Thing", &names, Rc::clone(&vtable));
        assert!(Rc::ptr_eq(&first, &second), "layouts should be shared");

        let different = registry.intern("Thing", &["other".to_string()], Rc::clone(&vtable));
        assert!(!Rc::ptr_eq(&first, &different));
        assert_eq!(first.field_names().to_vec(), vec!["a", "m", "z"]);
        assert_eq!(first.slot("a"), Some(0));
        assert_eq!(first.slot("z"), Some(2));
        assert!(Rc::ptr_eq(first.vtable(), &vtable));

        let mut instance = Instance::new(first);
        assert_eq!(instance.slots().to_vec(), vec![CompactValue::Null; 3]);
        instance.set_field("z", CompactValue::Int(30)).unwrap();
        instance.set_field("a", CompactValue::Int(10)).unwrap();
        assert_eq!(instance.get_field("a").unwrap(), &CompactValue::Int(10));
        assert_eq!(instance.get_slot(2).unwrap(), &CompactValue::Int(30));
        assert_eq!(
            instance.set_field("missing", CompactValue::Null),
            Err(CompactError::UnknownField("missing".to_string()))
        );
        assert_eq!(
            instance.get_slot(99),
            Err(CompactError::UnknownSlot(99))
        );
    }

    #[test]
    fn vtable_dispatch_by_u16_slot() {
        let vtable = Rc::new(VTable::new(vec![
            VTableEntry { name: "area".to_string(), method: 100 },
            VTableEntry { name: "move".to_string(), method: 101 },
            VTableEntry { name: "move".to_string(), method: 102 },
        ]));

        assert_eq!(vtable.len(), 3);
        assert!(!vtable.is_empty());
        assert_eq!(vtable.lookup(0).unwrap().method, 100);
        assert_eq!(vtable.lookup(2).unwrap().name, "move");
        assert_eq!(vtable.lookup(3), None);
        assert_eq!(vtable.slot_of("area"), Some(0));
        assert_eq!(vtable.slot_of("move"), Some(1));
        assert_eq!(vtable.slot_of("missing"), None);

        let mut registry = LayoutRegistry::default();
        let layout = registry.intern("Shape", &[], Rc::clone(&vtable));
        let left = Instance::new(Rc::clone(&layout));
        let right = Instance::new(layout);
        assert!(Rc::ptr_eq(left.layout().vtable(), right.layout().vtable()));
        assert_eq!(left.method(1).unwrap().method, 101);
        assert_eq!(right.method(1).unwrap().method, 101);
    }

    #[test]
    fn class_instance_conversion_uses_layout_and_vtable() {
        let mut fields = HashMap::new();
        fields.insert("y".to_string(), Value::Int(7));
        fields.insert("x".to_string(), Value::String(Rc::new("hi".to_string())));
        let mut method_map = HashMap::new();
        method_map.insert("area".to_string(), vec![10u16]);
        method_map.insert("move".to_string(), vec![20u16, 21u16]);

        let original = Value::ClassInstance {
            class_name: "Point".to_string(),
            fields: Rc::new(RefCell::new(fields)),
            vtable: method_map,
        };

        let compact = CompactValue::from_value(&original).unwrap();
        let CompactValue::Heap(heap) = &compact else {
            panic!("expected heap value");
        };
        let HeapValue::Instance(instance) = heap.as_ref() else {
            panic!("expected instance");
        };
        assert_eq!(instance.layout().class_name, "Point");
        assert_eq!(instance.layout().field_count(), 2);
        assert_eq!(instance.layout().slot("x"), Some(0));
        assert_eq!(instance.get_field("y").unwrap(), &CompactValue::Int(7));
        assert_eq!(instance.method(0).unwrap().method, 10);
        assert_eq!(instance.method(1).unwrap().method, 20);

        let back = compact.to_value().unwrap();
        let Value::ClassInstance {
            class_name,
            fields,
            vtable,
        } = back
        else {
            panic!("expected class instance");
        };
        assert_eq!(class_name, "Point");
        let borrowed = fields.borrow();
        assert_eq!(
            borrowed.get("x"),
            Some(&Value::String(Rc::new("hi".to_string())))
        );
        assert_eq!(borrowed.get("y"), Some(&Value::Int(7)));
        drop(borrowed);
        assert_eq!(vtable.get("area"), Some(&vec![10u16]));
        assert_eq!(vtable.get("move"), Some(&vec![20u16, 21u16]));
    }

    #[test]
    fn unsupported_values_return_honest_errors() {
        let unsupported = vec![
            Value::Cell(Rc::new(RefCell::new(Value::Int(1)))),
            Value::Closure { func_idx: 0, upvalues: vec![] },
            Value::FileHandle(Rc::new(RefCell::new(None))),
            Value::Socket(Rc::new(RefCell::new(None))),
            Value::Listener(Rc::new(RefCell::new(None))),
        ];

        for value in unsupported {
            let error = CompactValue::from_value(&value).unwrap_err();
            assert!(matches!(&error, CompactError::NotRepresentable(_)));
            assert!(error.to_string().contains("not representable"));
        }

        let nested = Value::Array {
            elements: vec![Value::Int(1), Value::Cell(Rc::new(RefCell::new(Value::Int(2))))],
        };
        assert!(matches!(
            CompactValue::from_value(&nested),
            Err(CompactError::NotRepresentable(_))
        ));
    }

    #[test]
    fn compact_value_slot_is_smaller_than_value() {
        assert!(
            std::mem::size_of::<CompactValue>() < std::mem::size_of::<Value>(),
            "compact slot {} vs value slot {}",
            std::mem::size_of::<CompactValue>(),
            std::mem::size_of::<Value>()
        );
    }
}
