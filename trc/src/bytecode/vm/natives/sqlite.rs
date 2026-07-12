// Titrate Alpha 0.3 – bytecode virtual machine: SQLite native functions
// Precision in every step – richie-rich90454, 2026
//
// Real SQLite database support using the `rusqlite` crate (bundled SQLite).
// Connection objects are stored in a thread_local registry keyed by handle.
// Query results are fetched eagerly to avoid Statement lifetime issues.

use super::super::super::value::Value;
use rusqlite::Connection;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A query result set: column names + rows of values + current cursor position.
struct ResultSet {
    column_names: Vec<String>,
    rows: Vec<Vec<Value>>,
    cursor: usize,
}

thread_local! {
    static CONN_REGISTRY: RefCell<HashMap<i64, Connection>> = RefCell::new(HashMap::new());
    static RESULT_REGISTRY: RefCell<HashMap<i64, ResultSet>> = RefCell::new(HashMap::new());
    static NEXT_HANDLE: std::sync::atomic::AtomicI64 = const { std::sync::atomic::AtomicI64::new(1) };
}

fn get_handle() -> i64 {
    NEXT_HANDLE.with(|h| h.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
}

/// Extract elements from a Titrate ArrayList (stored as _elements field in ClassInstance).
fn extract_array_elements(val: &Value) -> Vec<Value> {
    match val {
        Value::Array { elements } => elements.clone(),
        Value::ClassInstance { fields, .. } => {
            let fields = fields.borrow();
            if let Some(Value::Array { elements }) = fields.get("_elements") {
                elements.clone()
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}

/// Convert a rusqlite ValueRef to a Titrate Value.
fn rusqlite_to_value(val: &rusqlite::types::ValueRef) -> Value {
    use rusqlite::types::ValueRef;
    match val {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::Long(*i),
        ValueRef::Real(f) => Value::Double(*f),
        ValueRef::Text(s) => Value::String(Rc::new(String::from_utf8_lossy(s).to_string())),
        ValueRef::Blob(b) => Value::String(Rc::new(String::from_utf8_lossy(b).to_string())),
    }
}

pub(crate) fn native_sqlite_open(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Sqlite_open: expected a string path argument".to_string()),
    };

    let conn = Connection::open(&path)
        .map_err(|e| format!("Sqlite_open: failed to open '{}': {}", path, e))?;

    let handle = get_handle();
    CONN_REGISTRY.with(|r| {
        r.borrow_mut().insert(handle, conn);
    });
    Ok(Value::Long(handle))
}

pub(crate) fn native_sqlite_execute(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Sqlite_execute: expected 2 arguments (handle, sql)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let sql = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Sqlite_execute: expected a string sql argument".to_string()),
    };

    CONN_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let conn = registry.get_mut(&handle)
            .ok_or_else(|| "Sqlite_execute: invalid connection handle".to_string())?;

        conn.execute(&sql, [])
            .map_err(|e| format!("Sqlite_execute: SQL error: {}", e))?;

        Ok(Value::Void)
    })
}

pub(crate) fn native_sqlite_query(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Sqlite_query: expected 2 arguments (handle, sql)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let sql = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Sqlite_query: expected a string sql argument".to_string()),
    };

    CONN_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let conn = registry.get_mut(&handle)
            .ok_or_else(|| "Sqlite_query: invalid connection handle".to_string())?;

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("Sqlite_query: failed to prepare statement: {}", e))?;

        let column_count = stmt.column_count();
        let column_names: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).map(|s| s.to_string()).unwrap_or_default())
            .collect();

        let mut rows = stmt.query([])
            .map_err(|e| format!("Sqlite_query: failed to execute query: {}", e))?;

        let mut result_rows = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let mut row_values = Vec::with_capacity(column_count);
            for col in 0..column_count {
                let val = row.get_ref(col)
                    .map_err(|e| format!("Sqlite_query: failed to get column {}: {}", col, e))?;
                row_values.push(rusqlite_to_value(&val));
            }
            result_rows.push(row_values);
        }

        let rs_handle = get_handle();
        RESULT_REGISTRY.with(|r| {
            r.borrow_mut().insert(rs_handle, ResultSet {
                column_names,
                rows: result_rows,
                cursor: 0,
            });
        });

        Ok(Value::Long(rs_handle))
    })
}

pub(crate) fn native_sqlite_close(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Sqlite_close: expected a handle argument".to_string()),
    };

    CONN_REGISTRY.with(|r| {
        r.borrow_mut().remove(&handle);
    });
    Ok(Value::Void)
}

pub(crate) fn native_sqlite_last_insert_id(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Sqlite_lastInsertId: expected a handle argument".to_string()),
    };

    CONN_REGISTRY.with(|r| {
        let registry = r.borrow();
        let conn = registry.get(&handle)
            .ok_or_else(|| "Sqlite_lastInsertId: invalid connection handle".to_string())?;
        Ok(Value::Long(conn.last_insert_rowid()))
    })
}

pub(crate) fn native_sqlite_next_row(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Sqlite_nextRow: expected a handle argument".to_string()),
    };

    RESULT_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let rs = registry.get_mut(&handle)
            .ok_or_else(|| "Sqlite_nextRow: invalid result set handle".to_string())?;

        if rs.cursor < rs.rows.len() {
            rs.cursor += 1;
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Bool(false))
        }
    })
}

pub(crate) fn native_sqlite_get_int(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Sqlite_getInt: expected 2 arguments (handle, col)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let col = args[1].to_i64().unwrap_or(0) as usize;

    RESULT_REGISTRY.with(|r| {
        let registry = r.borrow();
        let rs = registry.get(&handle)
            .ok_or_else(|| "Sqlite_getInt: invalid result set handle".to_string())?;

        let row_idx = rs.cursor - 1;
        if row_idx >= rs.rows.len() {
            return Err("Sqlite_getInt: no current row".to_string());
        }
        if col >= rs.rows[row_idx].len() {
            return Err(format!("Sqlite_getInt: column {} out of bounds", col));
        }

        let val = &rs.rows[row_idx][col];
        Ok(Value::Long(val.to_i64().unwrap_or(0)))
    })
}

pub(crate) fn native_sqlite_get_string(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Sqlite_getString: expected 2 arguments (handle, col)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let col = args[1].to_i64().unwrap_or(0) as usize;

    RESULT_REGISTRY.with(|r| {
        let registry = r.borrow();
        let rs = registry.get(&handle)
            .ok_or_else(|| "Sqlite_getString: invalid result set handle".to_string())?;

        let row_idx = rs.cursor - 1;
        if row_idx >= rs.rows.len() {
            return Err("Sqlite_getString: no current row".to_string());
        }
        if col >= rs.rows[row_idx].len() {
            return Err(format!("Sqlite_getString: column {} out of bounds", col));
        }

        Ok(rs.rows[row_idx][col].clone())
    })
}

pub(crate) fn native_sqlite_get_double(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Sqlite_getDouble: expected 2 arguments (handle, col)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let col = args[1].to_i64().unwrap_or(0) as usize;

    RESULT_REGISTRY.with(|r| {
        let registry = r.borrow();
        let rs = registry.get(&handle)
            .ok_or_else(|| "Sqlite_getDouble: invalid result set handle".to_string())?;

        let row_idx = rs.cursor - 1;
        if row_idx >= rs.rows.len() {
            return Err("Sqlite_getDouble: no current row".to_string());
        }
        if col >= rs.rows[row_idx].len() {
            return Err(format!("Sqlite_getDouble: column {} out of bounds", col));
        }

        let val = &rs.rows[row_idx][col];
        match val {
            Value::Double(d) => Ok(Value::Double(*d)),
            Value::Long(l) => Ok(Value::Double(*l as f64)),
            Value::Int(i) => Ok(Value::Double(*i as f64)),
            _ => Ok(Value::Double(0.0)),
        }
    })
}

pub(crate) fn native_sqlite_column_count(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Sqlite_columnCount: expected a handle argument".to_string()),
    };

    RESULT_REGISTRY.with(|r| {
        let registry = r.borrow();
        let rs = registry.get(&handle)
            .ok_or_else(|| "Sqlite_columnCount: invalid result set handle".to_string())?;
        Ok(Value::Int(rs.column_names.len() as i32))
    })
}

pub(crate) fn native_sqlite_column_name(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Sqlite_columnName: expected 2 arguments (handle, col)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let col = args[1].to_i64().unwrap_or(0) as usize;

    RESULT_REGISTRY.with(|r| {
        let registry = r.borrow();
        let rs = registry.get(&handle)
            .ok_or_else(|| "Sqlite_columnName: invalid result set handle".to_string())?;

        if col >= rs.column_names.len() {
            return Err(format!("Sqlite_columnName: column {} out of bounds", col));
        }

        Ok(Value::String(Rc::new(rs.column_names[col].clone())))
    })
}

pub(crate) fn native_sqlite_close_result(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Sqlite_closeResult: expected a handle argument".to_string()),
    };

    RESULT_REGISTRY.with(|r| {
        r.borrow_mut().remove(&handle);
    });
    Ok(Value::Void)
}

pub(crate) fn native_sqlite_execute_prepared(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Sqlite_executePrepared: expected 3 arguments (handle, sql, params)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let sql = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Sqlite_executePrepared: expected a string sql argument".to_string()),
    };

    let params = extract_array_elements(&args[2]);

    CONN_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let conn = registry.get_mut(&handle)
            .ok_or_else(|| "Sqlite_executePrepared: invalid connection handle".to_string())?;

        let params_vec: Vec<Box<dyn rusqlite::ToSql>> = params.iter().map(|v| {
            match v {
                Value::Int(i) => Box::new(*i as i64) as Box<dyn rusqlite::ToSql>,
                Value::Long(l) => Box::new(*l) as Box<dyn rusqlite::ToSql>,
                Value::Double(d) => Box::new(*d) as Box<dyn rusqlite::ToSql>,
                Value::Float(f) => Box::new(*f as f64) as Box<dyn rusqlite::ToSql>,
                Value::Bool(b) => Box::new(*b as i64) as Box<dyn rusqlite::ToSql>,
                Value::String(s) => Box::new(s.as_str().to_string()) as Box<dyn rusqlite::ToSql>,
                Value::Null => Box::new(rusqlite::types::Null) as Box<dyn rusqlite::ToSql>,
                _ => Box::new(v.display_string()) as Box<dyn rusqlite::ToSql>,
            }
        }).collect();

        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| format!("Sqlite_executePrepared: execution error: {}", e))?;

        Ok(Value::Void)
    })
}

pub(crate) fn native_sqlite_backup(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Sqlite_backup: expected 2 arguments (handle, targetPath)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let target_path = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Sqlite_backup: expected a string targetPath argument".to_string()),
    };

    CONN_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let conn = registry.get_mut(&handle)
            .ok_or_else(|| "Sqlite_backup: invalid connection handle".to_string())?;

        let mut target_conn = Connection::open(&target_path)
            .map_err(|e| format!("Sqlite_backup: failed to open target: {}", e))?;

        let backup = rusqlite::backup::Backup::new(conn, &mut target_conn)
            .map_err(|e| format!("Sqlite_backup: failed to create backup: {}", e))?;

        backup.run_to_completion(100, std::time::Duration::from_millis(10), None)
            .map_err(|e| format!("Sqlite_backup: backup failed: {}", e))?;

        Ok(Value::Void)
    })
}
