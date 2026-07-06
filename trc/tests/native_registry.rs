//! Integration test: native function registry 1:1:1 correspondence.
//!
//! Verifies that the three sources of truth for native functions are in
//! exact 1:1:1 correspondence:
//!   1. Wrapper definitions in `titrate_native/src/wrappers.rs`
//!      (`pub extern "C" fn titrate_<name>`)
//!   2. VM registrations in `trc/src/bytecode/vm/natives/lookup.rs`
//!      (`"<name>" => Some(...)`)
//!   3. C header declarations in `titrate_native/titrate_native.h`
//!      (`TitrateValue titrate_<name>(const TitrateValue *args, size_t arg_count);`)
//!
//! The natives module (`trc/src/bytecode/vm/natives/mod.rs`) declares
//! `mod lookup;` and re-exports `lookup_builtin_native`; the actual
//! registration table lives in `lookup.rs`.
//!
//! The only exception is "println", which is a direct helper in `lib.rs`
//! (with a distinct signature) rather than a uniform wrapper in
//! `wrappers.rs`. It is excluded from the 1:1:1 check.

use std::collections::BTreeSet;
use std::env;
use std::path::PathBuf;

/// Names implemented as direct helpers in `titrate_native/src/lib.rs`,
/// not as uniform wrappers in `wrappers.rs`. These are registered in the
/// VM but have no wrapper and no uniform-signature header declaration.
const DIRECT_HELPERS: &[&str] = &["println"];

/// Locate the workspace root by walking up from `CARGO_MANIFEST_DIR`.
fn workspace_root() -> PathBuf {
    let manifest = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR should be set by cargo");
    PathBuf::from(manifest)
        .parent()
        .map(|p| p.to_path_buf())
        .expect("trc should be inside the workspace")
}

/// Extract wrapper names from `titrate_native/src/wrappers.rs`.
///
/// Matches lines containing `pub extern "C" fn titrate_<name>(` and
/// captures `<name>` (the identifier after the `titrate_` prefix).
fn collect_wrappers(src: &str) -> BTreeSet<String> {
    let marker = "pub extern \"C\" fn titrate_";
    let mut set = BTreeSet::new();
    for line in src.lines() {
        let Some(pos) = line.find(marker) else {
            continue;
        };
        let after = &line[pos + marker.len()..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            set.insert(name);
        }
    }
    set
}

/// Extract registered native names from
/// `trc/src/bytecode/vm/natives/lookup.rs`.
///
/// Matches match-arm patterns of the form `"<name>" => Some(`.
fn collect_registered(src: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for line in src.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('"') {
            continue;
        }
        let rest = &trimmed[1..];
        let Some(end) = rest.find('"') else {
            continue;
        };
        let name = &rest[..end];
        let after_quote = &rest[end + 1..];
        if !after_quote.trim_start().starts_with("=>") {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

/// Extract wrapper declaration names from
/// `titrate_native/titrate_native.h`.
///
/// Matches lines of the form
/// `TitrateValue titrate_<name>(const TitrateValue *args, size_t arg_count);`
/// — the uniform wrapper signature. Direct helpers (with distinct
/// signatures like `void titrate_println(...)`) are not matched.
fn collect_header_decls(src: &str) -> BTreeSet<String> {
    let prefix = "TitrateValue titrate_";
    let suffix = "arg_count);";
    let mut set = BTreeSet::new();
    for line in src.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with(prefix) || !trimmed.ends_with(suffix) {
            continue;
        }
        let after = &trimmed[prefix.len()..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            set.insert(name);
        }
    }
    set
}

#[test]
fn native_registry_1_to_1_to_1() {
    let root = workspace_root();

    let wrappers_src = std::fs::read_to_string(
        root.join("titrate_native").join("src").join("wrappers.rs"),
    )
    .expect("failed to read wrappers.rs");

    let lookup_src = std::fs::read_to_string(
        root.join("trc")
            .join("src")
            .join("bytecode")
            .join("vm")
            .join("natives")
            .join("lookup.rs"),
    )
    .expect("failed to read lookup.rs");

    let header_src =
        std::fs::read_to_string(root.join("titrate_native").join("titrate_native.h"))
            .expect("failed to read titrate_native.h");

    let wrappers = collect_wrappers(&wrappers_src);
    let mut registered = collect_registered(&lookup_src);
    let header_decls = collect_header_decls(&header_src);

    // Guard against a silent parsing failure that would produce empty
    // sets and trivially "pass" the correspondence check.
    assert!(
        !wrappers.is_empty(),
        "parsed zero wrappers — wrappers.rs parser is broken",
    );
    assert!(
        !registered.is_empty(),
        "parsed zero registered names — lookup.rs parser is broken",
    );
    assert!(
        !header_decls.is_empty(),
        "parsed zero header declarations — header parser is broken",
    );

    // Remove direct-helper names from the registered set — they are
    // intentionally not wrappers and are excluded from the 1:1:1 check.
    for dh in DIRECT_HELPERS {
        registered.remove(*dh);
    }

    let mut errors: Vec<String> = Vec::new();

    for name in wrappers.difference(&registered) {
        errors.push(format!("Wrapper {} is not registered in VM", name));
    }
    for name in wrappers.difference(&header_decls) {
        errors.push(format!("Wrapper {} is not declared in header", name));
    }
    for name in registered.difference(&wrappers) {
        errors.push(format!("Registered name {} has no wrapper", name));
    }
    for name in header_decls.difference(&wrappers) {
        errors.push(format!("Header declaration {} has no wrapper", name));
    }

    if !errors.is_empty() {
        panic!(
            "Native registry 1:1:1 mismatch ({} wrappers, {} registered, \
             {} header decls):\n  - {}",
            wrappers.len(),
            registered.len(),
            header_decls.len(),
            errors.join("\n  - "),
        );
    }
}
