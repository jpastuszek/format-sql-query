/*!
Collection of types and helpers for building hopefully correctly escaped SQL queries.

Example usage
=============

```rust
use format_sql_query::*;

println!("SELECT {} FROM {} WHERE {} = {}", Column("foo bar".into()), SchemaTable("foo".into(), "baz".into()), Column("blah".into()), QuotedData("hello 'world' foo"));
// SELECT "foo bar" FROM foo.baz WHERE blah = 'hello ''world'' foo'
```

Design goals
============

* All objects will implement `Display` to get escaped and perhaps quoted formatting that can be used directly in SQL statements.
* Avoid allocations by making most types just wrappers around string slices.
* New-type patter that is used to construct an object out of strings and other objects.
* Generous `From` trait implementations to make it easy to construct objects from strings.
* All single field new-type objects will implement `.as_str()` to get original value.
* Types that are string slice wrappers implement `Copy` to make them easy to use.
* Types should implement `Eq` and `Ord`.
* New-type objects with more than one filed should have getters.
* When returning types make sure they don't reference self but the original string slice lifetime.

All objects are using base escaping rules wrappers:

* `ObjectConcat` for table names, schemas, columns etc.
* `QuotedDataConcat` for data values
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

/// Generic object like table, schema, column etc. based `ObjectConcat` escaping rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Object<'i>(pub &'i str);

impl<'i> Object<'i> {
    /// Gets original value.
    pub fn as_str(&self) -> &'i str {
        self.0
    }

    /// Gets object represented as quoted data.
    pub fn as_quoted_data(&self) -> QuotedDataConcatDisplay<'i> {
        QuotedDataConcatDisplay(Box::new([self.as_str()]))
    }
}

impl<'i> From<&'i str> for Object<'i> {
    fn from(value: &'i str) -> Object<'i> {
        Object(value.into())
    }
}

impl fmt::Display for Object<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ObjectConcat(&[self.0]).fmt(f)
    }
}

/// Strings and other data in single quotes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    pub fn as_str(&self) -> &'i str {
        self.0
    }
}

impl fmt::Display for QuotedData<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        QuotedDataConcat(&[self.0]).fmt(f)
    }
}

/// Wrapper around `QuotedData` that maps its content.
pub struct MapQuotedData<'i, F>(&'i str, F);

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Schema<'i>(pub Object<'i>);

impl<'i> Schema<'i> {
    /// Gets original value.
    pub fn as_str(&self) -> &'i str {
        self.0.as_str()
    }

    /// Gets object represented as quoted data.
    pub fn as_quoted_data(&self) -> QuotedDataConcatDisplay<'i> {
        self.0.as_quoted_data()
    }
}

impl<'i, O: Into<Object<'i>> > From<O> for Schema<'i> {
    fn from(value: O) -> Schema<'i> {
        Schema(value.into())
    }
}

impl fmt::Display for Schema<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents database table name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Table<'i>(pub Object<'i>);

impl<'i> Table<'i> {
    /// Constructs `SchemaTable` from this table and given schema.
    pub fn with_schema(self, schema: impl Into<Schema<'i>>) -> SchemaTable<'i> {
        SchemaTable(schema.into(), self)
    }

    /// Returns object implementing `Display` to format this table name with given postfix.
    pub fn with_postfix(&self, postfix: &'i str) -> ObjectConcatDisplay<'i> {
        ObjectConcatDisplay(Box::new([self.as_str(), postfix]))
    }

    /// Returns object implementing `Display` to format this table name with given postfix
    /// separated with given separator.
    pub fn with_postfix_sep(&self, postfix: &'i str, separator: &'i str) -> ObjectConcatDisplay<'i> {
        ObjectConcatDisplay(Box::new([self.as_str(), separator, postfix]))
    }

    /// Gets original value.
    pub fn as_str(&self) -> &'i str {
        self.0.as_str()
    }

    /// Gets object represented as quoted data.
    pub fn as_quoted_data(&self) -> QuotedDataConcatDisplay<'i> {
        self.0.as_quoted_data()
    }
}

impl<'i, O: Into<Object<'i>> > From<O> for Table<'i> {
    fn from(table: O) -> Table<'i> {
        Table(table.into())
    }
}

impl fmt::Display for Table<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents table name in a schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SchemaTable<'i>(pub Schema<'i>, pub Table<'i>);

impl<'i> SchemaTable<'i> {
    /// Gets `Schema` part
    pub fn schema(&self) -> Schema<'i> {
        self.0
    }

    /// Gets `Table` part
    pub fn table(&self) -> Table<'i> {
        self.1
    }

    fn as_array(&self) -> [&'i str; 3] {
        [self.0.as_str(), ".", self.1.as_str()]
    }

    /// Returns object implementing `Display` to format this table name with given postfix.
    pub fn with_postfix(&self, postfix: &'i str) -> impl Display + 'i {
        let a = self.as_array();
        ObjectConcatDisplay(Box::new([a[0], a[1], a[2], postfix]))
    }

    /// Returns object implementing `Display` to format this table name with given postfix
    /// separated with given separator.
    pub fn with_postfix_sep(&self, postfix: &'i str, separator: &'i str) -> ObjectConcatDisplay<'i> {
        let a = self.as_array();
        ObjectConcatDisplay(Box::new([a[0], a[1], a[2], separator, postfix]))
    }

    /// Gets object represented as quoted data.
    pub fn as_quoted_data(&self) -> QuotedDataConcatDisplay<'i> {
        QuotedDataConcatDisplay(Box::new(self.as_array()))
    }
}

impl<'i, S: Into<Schema<'i>>, T: Into<Table<'i>>> From<(S, T)> for SchemaTable<'i> {
    fn from((schema, table): (S, T)) -> SchemaTable<'i> {
        SchemaTable(schema.into(), table.into())
    }
}

impl fmt::Display for SchemaTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ObjectConcat(&self.as_array()).fmt(f)
    }
}

/// Represents table column name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Column<'i>(pub Object<'i>);

impl<'i> Column<'i> {
    /// Gets original value.
    pub fn as_str(&self) -> &'i str {
        self.0.as_str()
    }

    /// Gets object represented as quoted data.
    pub fn as_quoted_data(&self) -> QuotedDataConcatDisplay<'i> {
        self.0.as_quoted_data()
    }
}

impl<'i, O: Into<Object<'i>>> From<O> for Column<'i> {
    fn from(value: O) -> Column<'i> {
        Column(value.into())
    }
}

impl fmt::Display for Column<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents column type for given SQL `Dialect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ColumnType<D: Dialect>(pub Object<'static>, pub PhantomData<D>);

impl<D: Dialect> ColumnType<D> {
    /// Gets original value.
    pub fn as_str(&self) -> &'static str {
        self.0.as_str()
    }
}

impl<D, O: Into<Object<'static>>> From<O> for ColumnType<D> where D: Dialect {
    fn from(column_type: O) -> ColumnType<D> {
        ColumnType(column_type.into(), PhantomData)
    }
}

impl<D: Dialect> fmt::Display for ColumnType<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents column name and type for given SQL `Dialect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ColumnSchema<'i, D: Dialect>(pub Column<'i>, pub ColumnType<D>);

impl<'i, D: Dialect> ColumnSchema<'i, D> {
    /// Gets `Column` part
    pub fn column(&self) -> &Column<'i> {
        &self.0
    }

    /// Gets `ColumnType` part
    pub fn column_type(&self) -> &ColumnType<D> {
        &self.1
    }
}

impl<'i, D: Dialect, C: Into<Column<'i>>, T: Into<ColumnType<D>>> From<(C, T)> for ColumnSchema<'i, D> {
    fn from((name, r#type): (C, T)) -> ColumnSchema<'i, D> {
        ColumnSchema(name.into(), r#type.into())
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
            r#"SELECT "foo bar" FROM foo.baz_quix WHERE blah = 'hello ''world'' foo'"#,
            &format!(
                "SELECT {} FROM {} WHERE {} = {}",
                Column("foo bar".into()),
                SchemaTable("foo".into(), "baz".into()).with_postfix("_quix"),
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
