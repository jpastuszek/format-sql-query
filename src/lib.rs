/*!
Collection of types and helpers for building hopefully correctly escaped SQL queries.

Example usage
=============

```rust
use format_sql_query::*;

println!("SELECT {} FROM {} WHERE {} = {}", Column("foo bar".into()), Table::with_schema("foo", "baz"), Column("blah".into()), QuotedData("hello 'world' foo"));
// SELECT "foo bar" FROM foo.baz WHERE blah = 'hello ''world'' foo'
```
 */
use itertools::Itertools;
use std::fmt;
use std::marker::PhantomData;

mod predicates;
pub use predicates::*;
mod data_type;
pub use data_type::*;

/// Object like table, schema, column etc.
///
/// Escaping rules:
/// * as is if does not contain " or space
/// * put in " and escape " with ""
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Object<'i>(pub &'i str);

impl<'i> From<&'i str> for Object<'i> {
    fn from(value: &'i str) -> Object<'i> {
        Object(value)
    }
}

impl fmt::Display for Object<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.contains("'") || self.0.contains("\\") {
            // MonetDB does not like ' or \ in column names
            return Err(fmt::Error);
        }

        if self.0.contains(" ") || self.0.contains("\"") {
            f.write_str("\"")?;
            for part in self.0.split("\"").intersperse("\"\"") {
                f.write_str(part)?;
            }
            f.write_str("\"")?;
        } else {
            f.write_str(self.0)?;
        }
        Ok(())
    }
}

/// Strings and other data in single quotes.
///
/// Escaping rules:
/// * put in ' and escape ' with ''
/// * escape / with //
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuotedData<'i>(pub &'i str);

impl<'i> From<&'i str> for QuotedData<'i> {
    fn from(value: &'i str) -> QuotedData<'i> {
        QuotedData(value)
    }
}

impl<'i> QuotedData<'i> {
    pub fn map<F>(self, f: F) -> MapQuotedData<'i, F>
    where
        F: Fn(&'i str) -> String,
    {
        MapQuotedData(self.0, f)
    }
}

impl fmt::Display for QuotedData<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("'")?;
        for part in self.0.split("'").intersperse("''") {
            for part in part.split("\\").intersperse("\\\\") {
                f.write_str(part)?;
            }
        }
        f.write_str("'")?;
        Ok(())
    }
}

/// Wrapper around `QuotedData` that maps its content.
pub struct MapQuotedData<'i, F>(pub &'i str, F);

impl<'i, F> fmt::Display for MapQuotedData<'i, F>
where
    F: Fn(&'i str) -> String,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let data = self.1(self.0);
        write!(f, "{}", QuotedData(&data))
    }
}

/// Represents database schema name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Schema<'i>(pub Object<'i>);

impl<'i, O> From<O> for Schema<'i> where O: Into<Object<'i>> {
    fn from(value: O) -> Schema<'i> {
        Schema(value.into())
    }
}

impl fmt::Display for Schema<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'i> Schema<'i> {
    pub fn new(name: impl Into<Object<'i>>) -> Schema<'i> {
        Schema(name.into())
    }
}

/// Represents table name with optional schema.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Table<'i> {
    pub schema: Option<Schema<'i>>,
    pub table: Object<'i>,
}

impl<'i> Table<'i> {
    pub fn with_schema(schema: impl Into<Schema<'i>>, table: impl Into<Object<'i>>) -> Table<'i> {
        Table {
            schema: Some(schema.into()),
            table: table.into(),
        }
    }

    pub fn new(table: impl Into<Object<'i>>) -> Table<'i> {
        Table {
            schema: None,
            table: table.into(),
        }
    }

    pub fn and_schema(self, schema: impl Into<Schema<'i>>) -> Table<'i> {
        Table {
            schema: Some(schema.into()),
            table: self.table,
        }
    }
}

impl<'i, T> From<T> for Table<'i> where T: Into<Object<'i>> {
    fn from(table: T) -> Table<'i> {
        Table {
            schema: None,
            table: table.into(),
        }
    }
}

impl<'i, S, T> From<(S, T)> for Table<'i> where S: Into<Schema<'i>>, T: Into<Object<'i>> {
    fn from((schema, table): (S, T)) -> Table<'i> {
        Table {
            schema: Some(schema.into()),
            table: table.into(),
        }
    }
}

impl fmt::Display for Table<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(schema) = self.schema {
            write!(f, "{}", Object(&format!("{}.{}", schema.0, self.table.0)))
        } else {
            write!(f, "{}", self.table)
        }
    }
}

/// Represents table column name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Column<'i>(pub Object<'i>);

impl<'i, O> From<O> for Column<'i> where O: Into<Object<'i>> {
    fn from(value: O) -> Column<'i> {
        Column(value.into())
    }
}

impl fmt::Display for Column<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'i> Column<'i> {
    pub fn new(name: impl Into<Object<'i>>) -> Column<'i> {
        Column(name.into())
    }
}

/// Represents table column name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnType<D: Dialect>(pub Object<'static>, PhantomData<D>);

impl<D: Dialect> fmt::Display for ColumnType<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents table column name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnSchema<'i, D: Dialect>(pub Column<'i>, pub ColumnType<D>);

impl<'i, D, C> From<(C, ColumnType<D>)> for ColumnSchema<'i, D> where D: Dialect, C: Into<Column<'i>> {
    fn from((column_name, column_type): (C, ColumnType<D>)) -> ColumnSchema<'i, D> {
        ColumnSchema(column_name.into(), column_type)
    }
}

impl<D: Dialect> fmt::Display for ColumnSchema<'_, D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.0, self.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_select() {
        assert_eq!(
            r#"SELECT "foo bar" FROM foo.baz WHERE blah = 'hello ''world'' foo'"#,
            &format!(
                "SELECT {} FROM {} WHERE {} = {}",
                Column("foo bar".into()),
                Table::with_schema("foo", "baz"),
                Column("blah".into()),
                QuotedData("hello 'world' foo")
            )
        )
    }
}
