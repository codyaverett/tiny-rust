# 37-tiny-sql-db

A minimal SQL database engine in `no_std` Rust with an HTTP API. Supports CREATE TABLE, INSERT, SELECT, UPDATE, DELETE, and DROP TABLE with INT and TEXT column types.

## What it does

- Listens on port 7881
- Parses and executes SQL statements via HTTP POST
- Token-by-token SQL parser with direct execution (no AST)
- All storage via anonymous mmap (~660KB zeroed memory)

## New concepts

- **SQL tokenizer** -- keyword recognition, string literals with `''` escaping, integer parsing
- **Direct-execution SQL parser** -- no intermediate AST, executes during parse
- **Columnar value types** -- discriminated union with Null/Int/Text variants
- **WHERE clause filtering** -- single equality predicate on any column

## SQL syntax

```sql
CREATE TABLE name (col1 INT, col2 TEXT, ...)
INSERT INTO name VALUES (val1, 'val2', ...)
SELECT * FROM name [WHERE col = val]
SELECT col1, col2 FROM name [WHERE col = val]
UPDATE name SET col = val [WHERE col = val]
DELETE FROM name [WHERE col = val]
DROP TABLE name
```

## HTTP API

| Method | Path | Action |
|--------|------|--------|
| POST | `/sql` | Execute SQL (body = SQL statement) |
| GET | `/tables` | List table names |
| GET | `/stats` | Table count + per-table row counts |

SELECT output: tab-separated columns, newline-separated rows, header row first.

## Usage

```sh
cargo build --release
./target/release/tiny-sql-db &

curl -X POST localhost:7881/sql -d "CREATE TABLE users (id INT, name TEXT, age INT)"
curl -X POST localhost:7881/sql -d "INSERT INTO users VALUES (1, 'Alice', 30)"
curl -X POST localhost:7881/sql -d "INSERT INTO users VALUES (2, 'Bob', 25)"
curl -X POST localhost:7881/sql -d "SELECT * FROM users"
curl -X POST localhost:7881/sql -d "SELECT name, age FROM users WHERE age = 30"
curl -X POST localhost:7881/sql -d "UPDATE users SET age = 31 WHERE name = 'Alice'"
curl -X POST localhost:7881/sql -d "DELETE FROM users WHERE id = 2"
curl -X POST localhost:7881/sql -d "DROP TABLE users"
curl localhost:7881/tables
curl localhost:7881/stats
```

## Limitations

- Max 4 tables, 8 columns per table, 256 rows per table
- Column names max 32 bytes, TEXT values max 64 bytes
- WHERE supports single `col = val` equality only
- Single-threaded, one connection at a time
- In-memory only, state lost on restart
