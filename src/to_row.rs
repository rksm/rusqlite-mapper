/// A trait that maps a struct to a row in a database.
pub trait ToRow: Sized {
    type Params<'a>: rusqlite::Params
    where
        Self: 'a;

    fn table_name() -> &'static str;

    fn column_names() -> &'static [&'static str];

    /// Returns a list of (sql data type name, optional, primary key)
    fn sql_types() -> Vec<(String, bool, bool)>;

    fn to_params(&self) -> Self::Params<'_>;

    fn create_table_statement() -> String {
        let mut stmt = String::from("CREATE TABLE ");
        stmt.push_str(Self::table_name());
        stmt.push_str(" (");
        stmt.push_str(
            &Self::column_names()
                .iter()
                .zip(Self::sql_types().iter())
                .map(|(name, (ty, optional, primary_key))| {
                    let mut stmt = String::from(*name);
                    stmt.push(' ');
                    stmt.push_str(ty);
                    if *primary_key {
                        stmt.push_str(" PRIMARY KEY");
                    } else if !*optional {
                        stmt.push_str(" NOT NULL");
                    }
                    stmt
                })
                .collect::<Vec<_>>()
                .join(", "),
        );
        stmt.push(')');
        stmt
    }

    fn insert_stmt() -> String {
        let mut stmt = String::from("INSERT INTO ");
        stmt.push_str(Self::table_name());
        stmt.push_str(" (");
        stmt.push_str(&Self::column_names().to_vec().join(", "));
        stmt.push_str(") VALUES (");
        stmt.push_str(
            &Self::column_names()
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", "),
        );
        stmt.push(')');
        stmt
    }

    fn upsert_stmt(id: &str) -> String {
        let mut stmt = Self::insert_stmt();
        stmt.push_str(" ON CONFLICT (");
        stmt.push_str(id);
        stmt.push_str(") DO UPDATE SET ");
        stmt.push_str(
            &Self::column_names()
                .iter()
                .map(|name| {
                    let mut stmt = String::from(*name);
                    stmt.push_str(" = excluded.");
                    stmt.push_str(name);
                    stmt
                })
                .collect::<Vec<_>>()
                .join(", "),
        );
        stmt
    }
}

pub trait SqliteTypeInfo {
    fn sqlite_type() -> &'static str;

    fn optional() -> bool {
        false
    }
}

#[rustfmt::skip]
mod types_impl {
    use rusqlite::types::Null;

    use super::*;

    impl<T: SqliteTypeInfo + ToOwned + ?Sized> SqliteTypeInfo for std::borrow::Cow<'_, T> { fn sqlite_type() -> &'static str { T::sqlite_type() } }
    impl<T: SqliteTypeInfo + ?Sized> SqliteTypeInfo for Box<T> { fn sqlite_type() -> &'static str { T::sqlite_type() } }
    impl<T: SqliteTypeInfo + ?Sized> SqliteTypeInfo for std::rc::Rc<T> { fn sqlite_type() -> &'static str { T::sqlite_type() } }
    impl<T: SqliteTypeInfo + ?Sized> SqliteTypeInfo for std::sync::Arc<T> { fn sqlite_type() -> &'static str { T::sqlite_type() } }
    impl<T: SqliteTypeInfo> SqliteTypeInfo for Option<T> {
        fn sqlite_type() -> &'static str { T::sqlite_type() }
        fn optional() -> bool { true }
    }

    impl SqliteTypeInfo for Null { fn sqlite_type() -> &'static str { "NULL" } }
    impl SqliteTypeInfo for bool { fn sqlite_type() -> &'static str { "INTEGER" } }
    impl SqliteTypeInfo for i8 { fn sqlite_type() -> &'static str { "INTEGER" } }
    impl SqliteTypeInfo for i16 { fn sqlite_type() -> &'static str { "INTEGER" } }
    impl SqliteTypeInfo for i32 { fn sqlite_type() -> &'static str { "INTEGER" } }
    impl SqliteTypeInfo for i64 { fn sqlite_type() -> &'static str { "INTEGER" } }
    impl SqliteTypeInfo for isize { fn sqlite_type() -> &'static str { "INTEGER" } }
    impl SqliteTypeInfo for u8 { fn sqlite_type() -> &'static str { "INTEGER" } }
    impl SqliteTypeInfo for u16 { fn sqlite_type() -> &'static str { "INTEGER" } }
    impl SqliteTypeInfo for u32 { fn sqlite_type() -> &'static str { "INTEGER" } }
    impl SqliteTypeInfo for f32 { fn sqlite_type() -> &'static str { "REAL" } }
    impl SqliteTypeInfo for f64 { fn sqlite_type() -> &'static str { "REAL" } }
    impl SqliteTypeInfo for u64 { fn sqlite_type() -> &'static str { "INTEGER" } }
    impl SqliteTypeInfo for usize { fn sqlite_type() -> &'static str { "INTEGER" } }

    impl SqliteTypeInfo for String { fn sqlite_type() -> &'static str { "TEXT" } }
    impl SqliteTypeInfo for str { fn sqlite_type() -> &'static str { "TEXT" } }

    impl SqliteTypeInfo for Vec<u8> { fn sqlite_type() -> &'static str { "BLOB" } }
    impl<const N: usize> SqliteTypeInfo for [u8; N] { fn sqlite_type() -> &'static str { "BLOB" } }
    impl SqliteTypeInfo for [u8] { fn sqlite_type() -> &'static str { "BLOB" } }


    #[cfg(feature = "serde")]
    impl SqliteTypeInfo for serde_json::Value { fn sqlite_type() -> &'static str { "TEXT" } }

    #[cfg(feature = "chrono")]
    impl SqliteTypeInfo for chrono::NaiveDate { fn sqlite_type() -> &'static str { "TEXT" } }
    #[cfg(feature = "chrono")]
    impl SqliteTypeInfo for chrono::NaiveTime { fn sqlite_type() -> &'static str { "TEXT" } }
    #[cfg(feature = "chrono")]
    impl SqliteTypeInfo for chrono::NaiveDateTime { fn sqlite_type() -> &'static str { "TEXT" } }
    #[cfg(feature = "chrono")]
    impl SqliteTypeInfo for chrono::DateTime<chrono::Utc> { fn sqlite_type() -> &'static str { "TEXT" } }
    #[cfg(feature = "chrono")]
    impl SqliteTypeInfo for chrono::DateTime<chrono::FixedOffset> { fn sqlite_type() -> &'static str { "TEXT" } }

    #[cfg(feature = "url")]
    impl SqliteTypeInfo for url::Url { fn sqlite_type() -> &'static str { "TEXT" } }

    #[cfg(feature = "uuid")]
    impl SqliteTypeInfo for uuid::Uuid { fn sqlite_type() -> &'static str { "TEXT" } }

    // impl SqliteType for ZeroBlob { fn sqlite_type() -> &'static str { "BLOB" } }

}
