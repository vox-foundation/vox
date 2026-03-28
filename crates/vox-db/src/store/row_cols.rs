//! Indexed `row.get` extraction with uniform [`crate::store::types::StoreError::Db`] mapping.
//!
//! Pilot for reducing repetitive `row.get(n).map_err(|e| StoreError::Db(e.to_string()))` boilerplate.

/// Extract typed columns from a Turso row.
///
/// # Example
///
/// ```ignore
/// use vox_db::row_cols;
/// row_cols!(row; 0 => sid: String, 1 => mtype: String, 2 => mv: Option<f64>);
/// ```
#[macro_export]
macro_rules! row_cols {
    ($row:expr; $($idx:expr => $name:ident : $ty:ty),+ $(,)?) => {
        $(
            let $name: $ty = $row
                .get($idx)
                .map_err(|e| $crate::store::types::StoreError::Db(e.to_string()))?;
        )+
    };
}
