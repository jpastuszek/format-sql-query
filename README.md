[![Latest Version]][crates.io] [![Documentation]][docs.rs] ![License]

Collection of types and helpers for building hopefully correctly escaped SQL queries.

Example usage
=============

```rust
use format_sql_query::*;

println!("SELECT {} FROM {} WHERE {} = {}", Column("foo bar".into()), SchemaTable("foo".into(), "baz".into()), Column("blah".into()), QuotedData("hello 'world' foo"));
// SELECT "foo bar" FROM foo.baz WHERE blah = 'hello ''world'' foo'
```

[crates.io]: https://crates.io/crates/format-sql-query
[Latest Version]: https://img.shields.io/crates/v/format-sql-query.svg
[Documentation]: https://docs.rs/format-sql-query/badge.svg
[docs.rs]: https://docs.rs/format-sql-query
[License]: https://img.shields.io/crates/l/format-sql-query.svg
