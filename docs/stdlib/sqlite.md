# sqlite

The `tt.db` module provides an embedded SQL database backed by SQLite. SqliteConnection manages database connections and transactions, while SqliteResultSet handles query results.

```titrate
import tt::db::SqliteConnection;
import tt::db::SqliteResultSet;
```

## SqliteConnection

Embedded SQLite database connection.

- `fn init(path: string)` — open database
- `execute(sql: string): void` — execute SQL statement
- `query(sql: string): SqliteResultSet` — execute query
- `lastInsertId(): int` — get last insert rowid
- `close(): void` — close connection
- `beginTransaction(): void` — begin transaction
- `commit(): void` — commit transaction
- `rollback(): void` — rollback transaction
- `isClosed(): bool` — check if closed

```titrate
let db = new SqliteConnection("test.db");
db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)");
db.execute("INSERT INTO users (name, age) VALUES ('Alice', 30)");
let insertId: int = db.lastInsertId();

db.beginTransaction();
db.execute("INSERT INTO users (name, age) VALUES ('Bob', 25)");
db.commit();

let rs: SqliteResultSet = db.query("SELECT * FROM users");
while (rs.next()) {
    let name: string = rs.getString(1);
    let age: int = rs.getInt(2);
    io::println(name + " is " + Integer.toString(age));
}
rs.close();
db.close();
```

## SqliteResultSet

Result set from a SQL query.

- `next(): bool` — advance to next row
- `getInt(col: int): int` — get integer column
- `getString(col: int): string` — get string column
- `getDouble(col: int): double` — get double column
- `getColumnCount(): int` — get number of columns
- `getColumnName(col: int): string` — get column name
- `close(): void` — close result set

```titrate
let rs: SqliteResultSet = db.query("SELECT name, age FROM users");
while (rs.next()) {
    let colCount: int = rs.getColumnCount();
    let colName: string = rs.getColumnName(0);
    let name: string = rs.getString(0);
    let age: int = rs.getInt(1);
}
rs.close();
```

## Prepared Statements

- `Sqlite.prepare(db: SqliteDb, sql: string): PreparedStatement` — create prepared statement
- `PreparedStatement.bindInt(index: int, value: int): void` — bind integer parameter
- `PreparedStatement.bindDouble(index: int, value: double): void` — bind double parameter
- `PreparedStatement.bindString(index: int, value: string): void` — bind string parameter
- `PreparedStatement.bindNull(index: int): void` — bind NULL
- `PreparedStatement.execute(): void` — execute statement
- `PreparedStatement.executeQuery(): ResultSet` — execute query
- `PreparedStatement.reset(): void` — reset for reuse
- `PreparedStatement.close(): void` — close statement

## Transaction Control

- `Sqlite.beginTransaction(db: SqliteDb): void` — begin transaction
- `Sqlite.commit(db: SqliteDb): void` — commit transaction
- `Sqlite.rollback(db: SqliteDb): void` — rollback transaction
- `Sqlite.savepoint(db: SqliteDb, name: string): void` — create savepoint
- `Sqlite.release(db: SqliteDb, name: string): void` — release savepoint
- `Sqlite.rollbackTo(db: SqliteDb, name: string): void` — rollback to savepoint

## Blob I/O

- `Sqlite.blobOpen(db: SqliteDb, table: string, column: string, rowid: long, readOnly: bool): Blob` — open blob for I/O
- `Blob.read(offset: int, length: int): ArrayList<byte>` — read blob bytes
- `Blob.write(offset: int, data: ArrayList<byte>): void` — write blob bytes
- `Blob.close(): void` — close blob

## WAL Mode

- `Sqlite.enableWAL(db: SqliteDb): void` — enable Write-Ahead Logging
- `Sqlite.disableWAL(db: SqliteDb): void` — disable WAL (rollback to journal)
- `Sqlite.checkpoint(db: SqliteDb): void` — checkpoint WAL

## Backup API

- `Sqlite.backup(source: SqliteDb, dest: string): void` — backup database to file
- `Sqlite.restore(dest: SqliteDb, source: string): void` — restore database from file
