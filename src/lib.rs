/*!
Collection of types and helpers for building hopefully correctly escaped SQL queries.

Example usage
=============

```rust
use format_sql_query::*;

println!("SELECT {} FROM {} WHERE {} = {}", Column("foo bar".into()), SchemaTable::new("foo", "baz"), Column("blah".into()), QuotedData("hello 'world' foo"));
// SELECT "foo bar" FROM foo.baz WHERE blah = 'hello ''world'' foo'
```

Design
======

Constructiors can be used to build all object using `impl Into<>` for arguments so objects can be easily created form supported types.
Objects will also implement `From` traits if they are simple wrappers, including tupples.
This is so explicit conversons are flexible (using `Into`) and implicit conversions are precise.

If type wraps more than one object fields will be named, otherwise new-type patter will be used.

All new-type objects will implement `.as_str()` to get original value.
All objects will implement `Display` to get escaped and perhaps quoted value that can be used in SQL statement.

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
/// * as-is, if does not contain " or space
/// * surround " and escape " with ""
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Object<'i>(pub &'i str);

impl<'i> Object<'i> {
    pub fn new(obj: impl Into<&'i str>) -> Object<'i> {
        Object(obj.into())
    }

    /// Gets original value.
    pub fn as_str(&self) -> &str {
        self.0
    }
}

impl<'i> From<&'i str> for Object<'i> {
    fn from(value: &'i str) -> Object<'i> {
        Object::new(value)
    }
}

impl fmt::Display for Object<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ObjectConcat(&[self.0]).fmt(f)
    }
}

/// Concatenation of strings with `Object`s escaping rules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectConcat<'i>(pub &'i [&'i str]);

impl fmt::Display for ObjectConcat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.iter().any(|o| o.contains("'") || o.contains("\\")) {
            // MonetDB does not like ' or \ in column names
            return Err(fmt::Error);
        }

        if self.0.iter().any(|o| o.contains(" ") || o.contains("\"")) {
            f.write_str("\"")?;
            for part in self.0.iter().flat_map(|o| o.split("\"").intersperse("\"\"")) {
                f.write_str(part)?;
            }
            f.write_str("\"")?;
        } else {
            for o in self.0.iter() {
                f.write_str(o)?;
            }
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

    /// Gets original value.
    pub fn as_str(&self) -> &str {
        self.0
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
        QuotedData(&data).fmt(f)
    }
}

/// Represents database schema name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Schema<'i>(pub Object<'i>);

impl<'i> Schema<'i> {
    pub fn new(name: impl Into<Object<'i>>) -> Schema<'i> {
        Schema(name.into())
    }

    /// Gets original value.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<'i, O> From<O> for Schema<'i> where O: Into<Object<'i>> {
    fn from(value: O) -> Schema<'i> {
        Schema(value.into())
    }
}

impl fmt::Display for Schema<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents table name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Table<'i>(Object<'i>);

impl<'i> Table<'i> {
    pub fn new(table: impl Into<Object<'i>>) -> Table<'i> {
        Table(table.into())
    }

    pub fn with_schema(self, schema: impl Into<Schema<'i>>) -> SchemaTable<'i> {
        SchemaTable::new(schema.into(), self)
    }

    /// Gets original value.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<'i, T> From<T> for Table<'i> where T: Into<Object<'i>> {
    fn from(table: T) -> Table<'i> {
        Table::new(table)
    }
}

impl fmt::Display for Table<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents table name in a schema.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SchemaTable<'i> {
    pub schema: Schema<'i>,
    pub table: Table<'i>,
}

impl<'i> SchemaTable<'i> {
    pub fn new(schema: impl Into<Schema<'i>>, table: impl Into<Table<'i>>) -> SchemaTable<'i> {
        SchemaTable {
            schema: schema.into(),
            table: table.into(),
        }
    }
}

impl<'i, S, T> From<(S, T)> for SchemaTable<'i> where S: Into<Schema<'i>>, T: Into<Table<'i>> {
    fn from((schema, table): (S, T)) -> SchemaTable<'i> {
        SchemaTable::new(schema, table)
    }
}

impl fmt::Display for SchemaTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ObjectConcat(&[self.schema.as_str(), ".", self.table.as_str()]).fmt(f)
    }
}

/// Represents table column name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Column<'i>(pub Object<'i>);

impl<'i> Column<'i> {
    pub fn new(name: impl Into<Object<'i>>) -> Column<'i> {
        Column(name.into())
    }

    /// Gets original value.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<'i> From<Object<'i>> for Column<'i> {
    fn from(value: Object<'i>) -> Column<'i> {
        Column::new(value)
    }
}

impl fmt::Display for Column<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents table column name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnType<D: Dialect>(pub Object<'static>, PhantomData<D>);

impl<D: Dialect> ColumnType<D> {
    pub fn new(column_type: impl Into<Object<'static>>) -> ColumnType<D> {
        ColumnType(column_type.into(), PhantomData)
    }

    /// Gets original value.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<D> From<Object<'static>> for ColumnType<D> where D: Dialect {
    fn from(column_type: Object<'static>) -> ColumnType<D> {
        ColumnType::new(column_type)
    }
}

impl<D: Dialect> fmt::Display for ColumnType<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents table column name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnSchema<'i, D: Dialect> {
    pub name: Column<'i>,
    pub r#type: ColumnType<D>,
}

impl<'i, D: Dialect> ColumnSchema<'i, D> {
    pub fn new(name: impl Into<Column<'i>>, r#type: impl Into<ColumnType<D>>) -> ColumnSchema<'i, D> {
        ColumnSchema {
            name: name.into(),
            r#type: r#type.into(),
        }
    }
}

impl<'i, D: Dialect> From<(Column<'i>, ColumnType<D>)> for ColumnSchema<'i, D> {
    fn from((name, r#type): (Column<'i>, ColumnType<D>)) -> ColumnSchema<'i, D> {
        ColumnSchema::new(name, r#type)
    }
}

impl<D: Dialect> fmt::Display for ColumnSchema<'_, D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.name, self.r#type)
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
                SchemaTable::new("foo", "baz"),
                Column("blah".into()),
                QuotedData("hello 'world' foo")
            )
        )
    }

    #[test]
    fn build_object_concat() {
        assert_eq!(
            r#""hello ""world"" foo_""quix""""#,
            &format!(
                "{}",
                ObjectConcat(&[r#"hello "world" foo"#, r#"_"quix""#])
            )
        );

        assert_eq!(
            "foo_bar_baz",
            &format!(
                "{}",
                ObjectConcat(&["foo_", "bar", "_baz"])
            )
        );
    }
}
