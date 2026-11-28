//! Advanced container-method bindings for the WASM backend.
//!
//! The AST lowering already binds the basic `ArrayList` and `HashMap` methods
//! to the `list_*` / `map_*` host imports. This module is the shared catalog
//! used by the extended emission path: it resolves type-dependent method
//! names to concrete host imports, describes iteration plans, and rewrites
//! compound methods (`getOrDefault`, `putIfAbsent`) in terms of imports that
//! already exist in `runtime.rs`. No new host import is required.
//!
//! A method that has no correct lowering returns an `Err` (or `None` for the
//! optional lookup helpers). Callers must surface that as an explicit
//! `wasm:` diagnostic rather than emitting a guessed call.

use super::WTy;

/// Which family of host objects a method applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    List,
    Map,
}

/// A resolved basic container method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodPlan {
    pub container: ContainerKind,
    pub method: String,
    pub host: String,
    /// Logical Titrate arguments, excluding the receiver.
    pub arg_count: usize,
}

/// How to drive a `for (x in collection)` loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterationPlan {
    pub container: ContainerKind,
    pub size_host: String,
    pub get_host: String,
    /// Map iteration first materialises the key list via `map_keys`.
    pub needs_keys: bool,
}

/// Shape of a compound container-method rewrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeRewriteKind {
    Host(String),
    /// `contains ? get : default`
    GetOrDefault { contains: String, get: String },
    /// `contains ? put : null` (the put call supplies the unit result)
    PutIfAbsent { contains: String, put: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRewrite {
    pub kind: NativeRewriteKind,
}

/// Suffix used by the boxed-value host imports for a physical WASM type.
pub fn host_suffix(ty: WTy) -> &'static str {
    match ty {
        WTy::I64 => "i64",
        WTy::F64 => "f64",
        WTy::Bool => "bool",
        WTy::Str | WTy::Char => "str",
        WTy::Handle | WTy::Void | WTy::Unknown => "obj",
    }
}

fn container_kind(container: &str) -> Result<ContainerKind, String> {
    match container {
        "ArrayList" | "List" | "list" => Ok(ContainerKind::List),
        "HashMap" | "Map" | "map" => Ok(ContainerKind::Map),
        other => Err(format!("wasm: unknown container type '{}'", other)),
    }
}

/// Resolve a container method whose host import depends on a value type.
pub fn plan(container: &str, method: &str, ty: WTy) -> Result<MethodPlan, String> {
    let kind = container_kind(container)?;
    let suffix = host_suffix(ty);
    let (host, arg_count) = match (kind, method) {
        (ContainerKind::List, "add") => ("list_add".to_string(), 1),
        (ContainerKind::List, "get") => (format!("list_get_{}", suffix), 1),
        (ContainerKind::List, "set") => ("list_set".to_string(), 2),
        (ContainerKind::List, "size" | "length") => ("list_size".to_string(), 0),
        (ContainerKind::List, "isEmpty") => ("list_is_empty".to_string(), 0),
        (ContainerKind::List, "contains") => ("list_contains".to_string(), 1),
        (ContainerKind::List, "indexOf") => ("list_index_of".to_string(), 1),
        (ContainerKind::List, "lastIndexOf") => ("list_last_index_of".to_string(), 1),
        (ContainerKind::List, "remove") => (format!("list_remove_{}", suffix), 1),
        (ContainerKind::List, "removeAt") => ("list_remove_i64".to_string(), 1),
        (ContainerKind::List, "pop") => (format!("list_pop_{}", suffix), 0),
        (ContainerKind::List, "addAll") => ("list_add_all".to_string(), 1),
        (ContainerKind::List, "clear") => ("list_clear".to_string(), 0),
        (ContainerKind::List, "reverse") => ("list_reverse".to_string(), 0),
        (ContainerKind::List, "max") => (format!("list_max_{}", suffix), 0),
        (ContainerKind::List, "min") => (format!("list_min_{}", suffix), 0),
        (ContainerKind::List, "iterator") => ("iterator_from_list".to_string(), 0),
        (ContainerKind::Map, "put") => (format!("map_put_{}", suffix), 2),
        (ContainerKind::Map, "get") => (format!("map_get_{}_obj", suffix), 1),
        (ContainerKind::Map, "remove") => (format!("map_remove_{}", suffix), 1),
        (ContainerKind::Map, "size") => ("map_size".to_string(), 0),
        (ContainerKind::Map, "containsKey") => (format!("map_contains_{}", suffix), 1),
        (ContainerKind::Map, "containsValue") => (format!("map_contains_value_{}", suffix), 1),
        (ContainerKind::Map, "clear") => ("map_clear".to_string(), 0),
        (ContainerKind::Map, "keys") => ("map_keys".to_string(), 0),
        (ContainerKind::Map, "values") => ("map_values".to_string(), 0),
        (ContainerKind::List, "sort" | "binarySearch" | "subList") => {
            return Err(format!(
                "wasm: ArrayList.{} needs a host comparison routine and is not lowered",
                method
            ))
        }
        (ContainerKind::Map, method) => {
            return Err(format!("wasm: HashMap.{} has no host binding", method))
        }
        (ContainerKind::List, method) => {
            return Err(format!("wasm: ArrayList.{} has no host binding", method))
        }
    };
    Ok(MethodPlan {
        container: kind,
        method: method.to_string(),
        host,
        arg_count,
    })
}

/// A type-independent native alias, checked before the typed rewrites.
pub fn host_alias(name: &str) -> Option<&'static str> {
    Some(match name {
        "ArrayList.size" | "ArrayList.length" | "List.size" => "list_size",
        "ArrayList.isEmpty" | "List.isEmpty" => "list_is_empty",
        "ArrayList.clear" | "List.clear" => "list_clear",
        "ArrayList.reverse" | "List.reverse" => "list_reverse",
        "ArrayList.iterator" | "List.iterator" => "iterator_from_list",
        "HashMap.size" | "Map.size" => "map_size",
        "HashMap.clear" | "Map.clear" => "map_clear",
        "HashMap.keys" | "Map.keys" => "map_keys",
        "HashMap.values" | "Map.values" => "map_values",
        _ => return None,
    })
}

fn contains_host(key: WTy) -> Option<&'static str> {
    match key {
        WTy::Str | WTy::Char => Some("map_contains_str"),
        WTy::Bool => Some("map_contains_bool"),
        WTy::I64 => Some("map_contains_i64"),
        WTy::F64 => Some("map_contains_f64"),
        WTy::Handle | WTy::Void | WTy::Unknown => None,
    }
}

/// Resolve a typed `HashMap` / `ArrayList` native name into a rewrite.
pub fn plan_native_typed(name: &str, key: WTy, value: WTy) -> Option<NativeRewrite> {
    let contains = contains_host(key)?;
    let k = host_suffix(key);
    let v = host_suffix(value);
    let kind = match name {
        "HashMap.getOrDefault" | "Map.getOrDefault" => NativeRewriteKind::GetOrDefault {
            contains: contains.to_string(),
            get: format!("map_get_{}_{}", k, v),
        },
        "HashMap.putIfAbsent" | "Map.putIfAbsent" => NativeRewriteKind::PutIfAbsent {
            contains: contains.to_string(),
            put: format!("map_put_{}", k),
        },
        "HashMap.containsKey" | "Map.containsKey" => {
            NativeRewriteKind::Host(contains.to_string())
        }
        "HashMap.get" | "Map.get" => {
            NativeRewriteKind::Host(format!("map_get_{}_{}", k, v))
        }
        // `put`, `containsValue` and `putIfAbsent` take a four-word boxed
        // value; the typed TIR rewrite cannot synthesize that safely, so the
        // caller must report an explicit unmapped error instead.
        _ => return None,
    };
    Some(NativeRewrite { kind })
}

/// Build the host calls needed to iterate a collection. Strings are rejected:
/// the bytecode VM itself rejects `for` over a bare string, so there is no
/// parity target for such a loop.
pub fn iteration_plan(container: &str, element: WTy) -> Result<IterationPlan, String> {
    match container_kind(container)? {
        ContainerKind::List => Ok(IterationPlan {
            container: ContainerKind::List,
            size_host: "list_size".to_string(),
            get_host: format!("list_get_{}", host_suffix(element)),
            needs_keys: false,
        }),
        ContainerKind::Map => Ok(IterationPlan {
            container: ContainerKind::Map,
            size_host: "map_size".to_string(),
            get_host: format!("list_get_{}", host_suffix(element)),
            needs_keys: true,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_typed_list_and_map_methods() {
        assert_eq!(
            plan("ArrayList", "get", WTy::I64),
            Ok(MethodPlan {
                container: ContainerKind::List,
                method: "get".to_string(),
                host: "list_get_i64".to_string(),
                arg_count: 1,
            })
        );
        assert_eq!(plan("HashMap", "get", WTy::Str).unwrap().host, "map_get_str_obj");
        assert_eq!(plan("HashMap", "put", WTy::I64).unwrap().host, "map_put_i64");
        assert!(plan("ArrayList", "sort", WTy::I64).is_err());
        assert!(plan("TreeSet", "get", WTy::I64).is_err());
    }

    #[test]
    fn rewrites_compound_map_methods() {
        let rewrite = plan_native_typed("HashMap.getOrDefault", WTy::Str, WTy::I64).unwrap();
        assert_eq!(
            rewrite.kind,
            NativeRewriteKind::GetOrDefault {
                contains: "map_contains_str".to_string(),
                get: "map_get_str_i64".to_string(),
            }
        );
        let put = plan_native_typed("Map.putIfAbsent", WTy::I64, WTy::Str).unwrap();
        assert_eq!(
            put.kind,
            NativeRewriteKind::PutIfAbsent {
                contains: "map_contains_i64".to_string(),
                put: "map_put_i64".to_string(),
            }
        );
        assert!(plan_native_typed("HashMap.getOrDefault", WTy::Handle, WTy::I64).is_none());
    }

    #[test]
    fn iteration_plans_use_existing_imports() {
        let list = iteration_plan("ArrayList", WTy::F64).unwrap();
        assert_eq!(list.size_host, "list_size");
        assert_eq!(list.get_host, "list_get_f64");
        assert!(!list.needs_keys);
        let map = iteration_plan("HashMap", WTy::Bool).unwrap();
        assert_eq!(map.get_host, "list_get_bool");
        assert!(map.needs_keys);
        assert!(iteration_plan("TreeSet", WTy::I64).is_err());
    }

    #[test]
    fn type_independent_aliases() {
        assert_eq!(host_alias("ArrayList.size"), Some("list_size"));
        assert_eq!(host_alias("HashMap.keys"), Some("map_keys"));
        assert_eq!(host_alias("ArrayList.sort"), None);
    }
}
