// Titrate Alpha 0.2 – bytecode virtual machine: SQLite native function stubs
// Precision in every step – richie-rich90454, 2026
//
// These are stub implementations. To enable real SQLite support,
// add the `rusqlite` dependency to Cargo.toml.

use super::super::super::value::Value;

pub(crate) fn native_sqlite_open(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_execute(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_query(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_close(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_last_insert_id(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_next_row(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_get_int(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_get_string(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_get_double(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_column_count(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_column_name(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}

pub(crate) fn native_sqlite_close_result(_args: &[Value]) -> Result<Value, String> {
    Err("SQLite not available: add rusqlite dependency".to_string())
}
