---
title: csv
description: CSV reading, writing, dialect detection, and dictionary-style row access for Titrate.
---

# csv

The `tt.csv` module reads and writes comma-separated values and other delimiter-separated formats. It supports configurable delimiters, quote characters, headers, dialect detection, and dictionary-style row access.

```titrate
import tt::csv::CsvReader;
import tt::csv::CsvWriter;
import tt::csv::DictReader;
import tt::csv::DictWriter;
```

## CsvReader

Parse CSV text into rows of strings.

- `fn init()` — create a reader with default `,` delimiter, `"` quote, and header enabled
- `setDelimiter(d: string): void`
- `setQuote(q: string): void`
- `setHasHeader(h: bool): void`
- `parse(input: string): ArrayList<ArrayList<string>>`
- `skipLines(input: string, lines: int): string`
- `getColumn(rows: ArrayList<ArrayList<string>>, colIndex: int): ArrayList<string>`
- `getColumnByName(rows: ArrayList<ArrayList<string>>, colName: string): ArrayList<string>`
- `getHeaders(input: string): ArrayList<string>`
- `parseToMaps(input: string): ArrayList<HashMap<string, string>>`

```titrate
let csv: CsvReader = new CsvReader();
let input: string = "name,age\nAlice,30\nBob,25";
let rows: ArrayList<ArrayList<string>> = csv.parse(input);

io::println(rows.get(1).get(0));  // "Alice"

let nameColumn: ArrayList<string> = csv.getColumnByName(rows, "name");
for (name in nameColumn) {
    io::println(name);
}
```

## CsvWriter

Write rows of strings to CSV text.

- `fn init()`
- `write(rows: ArrayList<ArrayList<string>>): string`
- `writeWithHeaders(headers: ArrayList<string>, rows: ArrayList<ArrayList<string>>): string`

```titrate
let writer: CsvWriter = new CsvWriter();

let headers: ArrayList<string> = new ArrayList<string>();
headers.add("name");
headers.add("age");

let rows: ArrayList<ArrayList<string>> = new ArrayList<ArrayList<string>>();
let row: ArrayList<string> = new ArrayList<string>();
row.add("Alice");
row.add("30");
rows.add(row);

io::println(writer.writeWithHeaders(headers, rows));
// name,age
// Alice,30
```

## DictReader

Read CSV rows as `HashMap<string, string>` keyed by column name.

- `fn init(input: string, fieldnames: ArrayList<string>, delimiter: string, quote: string)`
- `fn init(input: string, fieldnames: ArrayList<string>)`
- `fn init(input: string)` — use the first row as field names
- `readRow(): HashMap<string, string>` — returns `null` when exhausted
- `readAll(): ArrayList<HashMap<string, string>>`
- `reset(): void`

```titrate
let dictReader: DictReader = new DictReader(input);
let records: ArrayList<HashMap<string, string>> = dictReader.readAll();
for (record in records) {
    io::println(record.get("name") + " is " + record.get("age"));
}
```

## DictWriter

Write rows from `HashMap<string, string>`.

- `fn init(fieldnames: ArrayList<string>, delimiter: string, quote: string, extrasaction: string)`
- `fn init(fieldnames: ArrayList<string>, delimiter: string, quote: string)`
- `fn init(fieldnames: ArrayList<string>)`
- `writeHeader(): string`
- `writeRow(map: HashMap<string, string>): string`
- `writeRows(list: ArrayList<HashMap<string, string>>): string`

```titrate
let fieldnames: ArrayList<string> = new ArrayList<string>();
fieldnames.add("name");
fieldnames.add("age");

let dictWriter: DictWriter = new DictWriter(fieldnames);
io::println(dictWriter.writeHeader());

let record: HashMap<string, string> = new HashMap<string, string>();
record.put("name", "Alice");
record.put("age", "30");
io::println(dictWriter.writeRow(record));
```

## Dialect and Sniffer

- `Dialect.init(delimiter: string, quoteChar: string, hasHeader: bool)`
- `Sniffer.init()`
- `Sniffer.sniff(sample: string): Dialect` — auto-detect delimiter, quote char, and header presence

```titrate
let sniffer: Sniffer = new Sniffer();
let dialect: Dialect = sniffer.sniff("a;b;c\n1;2;3");
io::println(dialect.delimiter);  // ";"
```

## Quoting Constants

| Constant | Value |
|----------|-------|
| `QUOTE_MINIMAL` | 0 |
| `QUOTE_ALL` | 1 |
| `QUOTE_NONNUMERIC` | 2 |
| `QUOTE_NONE` | 3 |
