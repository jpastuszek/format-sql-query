use std::marker::PhantomData;
use crate::ColumnType;

/// SQL dialect of a database.
pub trait Dialect: Clone + Copy {}

/// Provide SQL data types in given dialect corresponding to Rust types.
pub trait SqlDataType<D: Dialect> {
    /// Gets `ColumnType` for given `Dialect` corresponding to `Self`.
    fn sql_type() -> ColumnType<D>;
}

#[macro_export]
macro_rules! impl_sql_data_type {
    ($dialect:ty, $t:ty, $sql_t:literal) => {
        impl SqlDataType<$dialect> for $t {
            fn sql_type() -> ColumnType<$dialect> {
                ColumnType($sql_t.into(), PhantomData)
            }
        }
    }
}

/// SQL Server SQL dialect.
#[derive(Clone, Copy, Debug)]
pub struct SqlServerDialect;
impl Dialect for SqlServerDialect {}

impl_sql_data_type!(SqlServerDialect, bool, "BIT");
impl_sql_data_type!(SqlServerDialect, i8, "TINYINT");
impl_sql_data_type!(SqlServerDialect, i16, "SMALLINT");
impl_sql_data_type!(SqlServerDialect, i32, "INT");
impl_sql_data_type!(SqlServerDialect, i64, "BIGINT");
impl_sql_data_type!(SqlServerDialect, f32, "REAL");
impl_sql_data_type!(SqlServerDialect, f64, "FLOAT");
impl_sql_data_type!(SqlServerDialect, String, "NVARCHAR(4000)");

/// MonetDB SQL dialect.
#[derive(Clone, Copy, Debug)]
pub struct MonetDbDialect;
impl Dialect for MonetDbDialect {}

impl_sql_data_type!(MonetDbDialect, bool, "BOOLEAN");
impl_sql_data_type!(MonetDbDialect, i8, "TINYINT");
impl_sql_data_type!(MonetDbDialect, i16, "SMALLINT");
impl_sql_data_type!(MonetDbDialect, i32, "INT");
impl_sql_data_type!(MonetDbDialect, i64, "BIGINT");
impl_sql_data_type!(MonetDbDialect, f64, "DOUBLE");
impl_sql_data_type!(MonetDbDialect, String, "STRING");
