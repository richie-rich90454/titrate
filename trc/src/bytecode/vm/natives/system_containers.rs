// Container natives (ArrayList/HashMap shims for the LLVM backend)


use super::super::super::value::{Value, values_eq};
use std::rc::Rc;

pub(crate) fn native_arraylist_size(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Array { elements }) => Ok(Value::Int(elements.len() as i32)),
        Some(other) => Err(format!("ArrayList.size: expected array, got {:?}", other)),
        None => Err("ArrayList.size: expected 1 argument".to_string()),
    }
}

/// ArrayList_get(array, index) -> any
/// Returns the element at the given index.
pub(crate) fn native_arraylist_get(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("ArrayList.get: expected 2 arguments (array, index)".to_string());
    }
    let idx = match &args[1] {
        Value::Int(i) => *i as usize,
        Value::Long(l) => *l as usize,
        other => return Err(format!("ArrayList.get: expected integer index, got {:?}", other)),
    };
    match &args[0] {
        Value::Array { elements } => {
            if idx < elements.len() {
                Ok(elements[idx].clone())
            } else {
                Err(format!("ArrayList.get: index {} out of bounds (len {})", idx, elements.len()))
            }
        }
        other => Err(format!("ArrayList.get: expected array, got {:?}", other)),
    }
}

/// ArrayList_add(array, element) -> array
/// Appends an element and returns the updated array. The LLVM native bridge
/// passes containers by value ({i64, ptr} copies), so mutation must be
/// functional: the codegen stores the returned array back into the receiver.
pub(crate) fn native_arraylist_add(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("ArrayList.add: expected 2 arguments (array, element)".to_string());
    }
    match &args[0] {
        Value::Array { elements } => {
            let mut new_elements = elements.clone();
            new_elements.push(args[1].clone());
            Ok(Value::Array { elements: new_elements })
        }
        other => Err(format!("ArrayList.add: expected array, got {:?}", other)),
    }
}

/// ArrayList_new() -> array
pub(crate) fn native_arraylist_new(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Array { elements: vec![] })
}

/// ArrayList_set(array, index, element) -> array
/// Replaces the element at `index` and returns the updated array.
pub(crate) fn native_arraylist_set(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("ArrayList.set: expected 3 arguments".to_string());
    }
    let idx = match &args[1] {
        Value::Int(i) => *i as usize,
        Value::Long(l) => *l as usize,
        other => return Err(format!("ArrayList.set: expected integer index, got {:?}", other)),
    };
    match &args[0] {
        Value::Array { elements } => {
            if idx >= elements.len() {
                return Err(format!(
                    "ArrayList.set: index {} out of bounds (len {})",
                    idx,
                    elements.len()
                ));
            }
            let mut new_elements = elements.clone();
            new_elements[idx] = args[2].clone();
            Ok(Value::Array { elements: new_elements })
        }
        other => Err(format!("ArrayList.set: expected array, got {:?}", other)),
    }
}

/// ArrayList_remove(array, element) -> array
/// Removes the first occurrence of `element` and returns the updated array.
pub(crate) fn native_arraylist_remove(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("ArrayList.remove: expected 2 arguments".to_string());
    }
    match &args[0] {
        Value::Array { elements } => {
            let mut new_elements = elements.clone();
            if let Some(pos) = new_elements.iter().position(|e| values_eq(e, &args[1])) {
                new_elements.remove(pos);
            }
            Ok(Value::Array { elements: new_elements })
        }
        other => Err(format!("ArrayList.remove: expected array, got {:?}", other)),
    }
}

/// ArrayList_removeAt(array, index) -> array
/// Removes the element at `index` and returns the updated array.
pub(crate) fn native_arraylist_remove_at(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("ArrayList.removeAt: expected 2 arguments".to_string());
    }
    let idx = match &args[1] {
        Value::Int(i) => *i as usize,
        Value::Long(l) => *l as usize,
        other => return Err(format!("ArrayList.removeAt: expected integer index, got {:?}", other)),
    };
    match &args[0] {
        Value::Array { elements } => {
            let mut new_elements = elements.clone();
            if idx < new_elements.len() {
                new_elements.remove(idx);
            }
            Ok(Value::Array { elements: new_elements })
        }
        other => Err(format!("ArrayList.removeAt: expected array, got {:?}", other)),
    }
}

/// ArrayList_contains(array, element) -> bool
pub(crate) fn native_arraylist_contains(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("ArrayList.contains: expected 2 arguments".to_string());
    }
    match &args[0] {
        Value::Array { elements } => Ok(Value::Bool(elements.iter().any(|e| values_eq(e, &args[1])))),
        other => Err(format!("ArrayList.contains: expected array, got {:?}", other)),
    }
}

/// ArrayList_indexOf(array, element) -> int
pub(crate) fn native_arraylist_index_of(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("ArrayList.indexOf: expected 2 arguments".to_string());
    }
    match &args[0] {
        Value::Array { elements } => {
            let pos = elements.iter().position(|e| values_eq(e, &args[1]));
            Ok(Value::Int(pos.map(|i| i as i32).unwrap_or(-1)))
        }
        other => Err(format!("ArrayList.indexOf: expected array, got {:?}", other)),
    }
}

/// ArrayList_isEmpty(array) -> bool
pub(crate) fn native_arraylist_is_empty(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("ArrayList.isEmpty: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::Array { elements } => Ok(Value::Bool(elements.is_empty())),
        other => Err(format!("ArrayList.isEmpty: expected array, got {:?}", other)),
    }
}

/// ArrayList_clear(array) -> array
/// Returns an empty array.
pub(crate) fn native_arraylist_clear(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("ArrayList.clear: expected 1 argument".to_string());
    }
    Ok(Value::Array { elements: vec![] })
}

/// ArrayList_toString(array) -> string
pub(crate) fn native_arraylist_to_string(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("ArrayList.toString: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::Array { elements } => {
            let items: Vec<String> = elements.iter().map(|e| e.display_string()).collect();
            Ok(Value::String(Rc::new(format!("[{}]", items.join(", ")))))
        }
        other => Err(format!("ArrayList.toString: expected array, got {:?}", other)),
    }
}

/// HashMap_new() -> map
pub(crate) fn native_hashmap_new(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Array { elements: vec![] })
}

/// Read the entries of a bridge HashMap, stored as a `Value::Array` of
/// `[key, value]` pair arrays. (The bytecode VM uses a ClassInstance-backed
/// HashMap; these natives serve the LLVM bridge, which marshals HashMap as a
/// TitrateArray.)
fn hashmap_pairs(map: &Value) -> Result<Vec<Value>, String> {
    match map {
        Value::Array { elements } => Ok(elements.clone()),
        other => Err(format!("HashMap: expected array, got {:?}", other)),
    }
}

/// HashMap_size(map) -> int
pub(crate) fn native_hashmap_size(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("HashMap.size: expected 1 argument".to_string());
    }
    let pairs = hashmap_pairs(&args[0])?;
    Ok(Value::Int(pairs.len() as i32))
}

/// HashMap_get(map, key) -> any
pub(crate) fn native_hashmap_get(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("HashMap.get: expected 2 arguments".to_string());
    }
    let pairs = hashmap_pairs(&args[0])?;
    for p in &pairs {
        if let Value::Array { elements } = p {
            if elements.len() == 2 && values_eq(&elements[0], &args[1]) {
                return Ok(elements[1].clone());
            }
        }
    }
    Ok(Value::Void)
}

/// HashMap_put(map, key, value) -> map
pub(crate) fn native_hashmap_put(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("HashMap.put: expected 3 arguments".to_string());
    }
    let mut pairs = hashmap_pairs(&args[0])?;
    let mut found = false;
    for p in pairs.iter_mut() {
        if let Value::Array { elements } = p {
            if elements.len() == 2 && values_eq(&elements[0], &args[1]) {
                elements[1] = args[2].clone();
                found = true;
                break;
            }
        }
    }
    if !found {
        pairs.push(Value::Array { elements: vec![args[1].clone(), args[2].clone()] });
    }
    Ok(Value::Array { elements: pairs })
}

/// HashMap_containsKey(map, key) -> bool
pub(crate) fn native_hashmap_contains_key(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("HashMap.containsKey: expected 2 arguments".to_string());
    }
    let pairs = hashmap_pairs(&args[0])?;
    Ok(Value::Bool(pairs.iter().any(|p| {
        matches!(p, Value::Array { elements } if elements.len() == 2 && values_eq(&elements[0], &args[1]))
    })))
}

/// HashMap_containsValue(map, value) -> bool
pub(crate) fn native_hashmap_contains_value(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("HashMap.containsValue: expected 2 arguments".to_string());
    }
    let pairs = hashmap_pairs(&args[0])?;
    Ok(Value::Bool(pairs.iter().any(|p| {
        matches!(p, Value::Array { elements } if elements.len() == 2 && values_eq(&elements[1], &args[1]))
    })))
}

/// HashMap_remove(map, key) -> map
pub(crate) fn native_hashmap_remove(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("HashMap.remove: expected 2 arguments".to_string());
    }
    let mut pairs = hashmap_pairs(&args[0])?;
    let mut i = 0;
    while i < pairs.len() {
        let remove = matches!(&pairs[i], Value::Array { elements } if elements.len() == 2 && values_eq(&elements[0], &args[1]));
        if remove {
            pairs.remove(i);
        } else {
            i += 1;
        }
    }
    Ok(Value::Array { elements: pairs })
}

/// HashMap_keys(map) -> array
pub(crate) fn native_hashmap_keys(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("HashMap.keys: expected 1 argument".to_string());
    }
    let pairs = hashmap_pairs(&args[0])?;
    let mut keys = Vec::new();
    for p in &pairs {
        if let Value::Array { elements } = p {
            if let Some(k) = elements.first() {
                keys.push(k.clone());
            }
        }
    }
    Ok(Value::Array { elements: keys })
}

/// HashMap_values(map) -> array
pub(crate) fn native_hashmap_values(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("HashMap.values: expected 1 argument".to_string());
    }
    let pairs = hashmap_pairs(&args[0])?;
    let mut values = Vec::new();
    for p in &pairs {
        if let Value::Array { elements } = p {
            if elements.len() >= 2 {
                values.push(elements[1].clone());
            }
        }
    }
    Ok(Value::Array { elements: values })
}

/// HashMap_isEmpty(map) -> bool
pub(crate) fn native_hashmap_is_empty(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("HashMap.isEmpty: expected 1 argument".to_string());
    }
    let pairs = hashmap_pairs(&args[0])?;
    Ok(Value::Bool(pairs.is_empty()))
}

/// HashMap_clear(map) -> array
pub(crate) fn native_hashmap_clear(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("HashMap.clear: expected 1 argument".to_string());
    }
    Ok(Value::Array { elements: vec![] })
}

/// HashMap_toString(map) -> string
pub(crate) fn native_hashmap_to_string(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("HashMap.toString: expected 1 argument".to_string());
    }
    let pairs = hashmap_pairs(&args[0])?;
    let mut parts: Vec<String> = Vec::new();
    for p in &pairs {
        if let Value::Array { elements } = p {
            if elements.len() >= 2 {
                parts.push(format!("{:?}={:?}", elements[0], elements[1]));
            }
        }
    }
    Ok(Value::String(Rc::new(format!("{{{}}}", parts.join(", ")))))
}
