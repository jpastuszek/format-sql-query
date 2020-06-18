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

All objects are using base escaping rules:
* `ObjectConcat` for table names, schemas, columns etc.
* `QuotedData` for data values

 */
use itertools::Itertools;
use std::fmt::{self, Display};
use std::marker::PhantomData;

mod predicates;
pub use predicates::*;
mod data_type;
pub use data_type::*;

/// Concatenation of strings with object escaping rules.
///
/// Escaping rules:
/// * as-is, if does not contain " or space
/// * surround " and escape " with ""
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

//TODO: reimplement using const generics when stable
/// Owned variant of `ObjectConcat` to be returned as `impl Display`.
pub struct ObjectConcatDisplay<'i>(Box<[&'i str]>);

impl<'i> ObjectConcatDisplay<'i> {
    pub fn as_quoted_data(self) -> QuotedDataConcatDisplay<'i> {
        self.into()
    }
}

impl fmt::Display for ObjectConcatDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ObjectConcat(&self.0).fmt(f)
    }
}

/// Concatenation of strings with quoted data escaping rules.
///
/// Escaping rules:
/// * put in ' and escape ' with ''
/// * escape / with //
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuotedDataConcat<'i>(pub &'i [&'i str]);

impl fmt::Display for QuotedDataConcat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("'")?;
        for part in self.0.iter().flat_map(|o| o.split("'").intersperse("''")) {
            for part in part.split("\\").intersperse("\\\\") {
                f.write_str(part)?;
            }
        }
        f.write_str("'")?;
        Ok(())
    }
}

//TODO: reimplement using const generics when stable
/// Owned variant of `QuotedDataConcat` to be returned as `impl Display`.
pub struct QuotedDataConcatDisplay<'i>(Box<[&'i str]>);

impl fmt::Display for QuotedDataConcatDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        QuotedDataConcat(&self.0).fmt(f)
    }
}

impl<'i> From<ObjectConcatDisplay<'i>> for QuotedDataConcatDisplay<'i> {
    fn from(v: ObjectConcatDisplay<'i>) -> QuotedDataConcatDisplay<'i> {
        QuotedDataConcatDisplay(v.0)
    }
}

/// Strings and other data in single quotes.
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
        QuotedDataConcat(&[self.0]).fmt(f)
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

/// Generic object like table, schema, column etc. based `ObjectConcat` escaping rules.
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

    /// Gets objecte represented as quoted data.
    pub fn as_quoted_data(&'i self) -> QuotedDataConcatDisplay<'i> {
        QuotedDataConcatDisplay(Box::new([self.as_str()]))
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

    /// Gets objecte represented as quoted data.
    pub fn as_quoted_data(&'i self) -> QuotedDataConcatDisplay<'i> {
        self.0.as_quoted_data()
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

    pub fn with_postfix(&'i self, postfix: &'i str) -> ObjectConcatDisplay<'i> {
        ObjectConcatDisplay(Box::new([self.as_str(), postfix]))
    }

    pub fn with_postfix_sep(&'i self, postfix: &'i str, separator: &'i str) -> ObjectConcatDisplay<'i> {
        ObjectConcatDisplay(Box::new([self.as_str(), separator, postfix]))
    }

    /// Gets original value.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Gets objecte represented as quoted data.
    pub fn as_quoted_data(&'i self) -> QuotedDataConcatDisplay<'i> {
        self.0.as_quoted_data()
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

    fn as_array(&self) -> [&str; 3] {
        [self.schema.as_str(), ".", self.table.as_str()]
    }

    pub fn with_postfix(&'i self, postfix: &'i str) -> impl Display + 'i {
        let a = self.as_array();
        ObjectConcatDisplay(Box::new([a[0], a[1], a[2], postfix]))
    }

    pub fn with_postfix_sep(&'i self, postfix: &'i str, separator: &'i str) -> ObjectConcatDisplay<'i> {
        let a = self.as_array();
        ObjectConcatDisplay(Box::new([a[0], a[1], a[2], separator, postfix]))
    }

    /// Gets objecte represented as quoted data.
    pub fn as_quoted_data(&'i self) -> QuotedDataConcatDisplay<'i> {
        QuotedDataConcatDisplay(Box::new(self.as_array()))
    }
}

impl<'i, S, T> From<(S, T)> for SchemaTable<'i> where S: Into<Schema<'i>>, T: Into<Table<'i>> {
    fn from((schema, table): (S, T)) -> SchemaTable<'i> {
        SchemaTable::new(schema, table)
    }
}

impl fmt::Display for SchemaTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ObjectConcat(&self.as_array()).fmt(f)
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

    /// Gets objecte represented as quoted data.
    pub fn as_quoted_data(&'i self) -> QuotedDataConcatDisplay<'i> {
        self.0.as_quoted_data()
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
            r#"SELECT "foo bar" FROM foo.baz_quix WHERE blah = 'hello ''world'' foo'"#,
            &format!(
                "SELECT {} FROM {} WHERE {} = {}",
                Column("foo bar".into()),
                SchemaTable::new("foo", "baz").with_postfix("_quix"),
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
