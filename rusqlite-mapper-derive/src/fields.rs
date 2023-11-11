use darling::FromField;

/// A single field inside of a struct that derives `FromRow`
#[derive(Debug, FromField)]
#[darling(attributes(rusqlite), forward_attrs(allow, doc, cfg))]
pub(crate) struct SqliteField {
    /// The identifier of this field.
    pub(crate) ident: Option<syn::Ident>,
    /// The type specified in this field.
    pub(crate) ty: syn::Type,
    /// Wether to flatten this field. Flattening means calling the `FromRow` implementation
    /// of `self.ty` instead of extracting it directly from the row.
    #[darling(default)]
    pub(crate) flatten: bool,
    /// Can only be used in combination with flatten. Will prefix all fields of the nested struct
    /// with this string. Can be useful for joins with overlapping names.
    pub(crate) prefix: Option<String>,
    /// Optionaly use this type as the target for `FromRow` or `FromSql`, and then
    /// call `TryFrom::try_from` to convert it the `self.ty`.
    pub(crate) try_from: Option<String>,
    /// Optionaly use this type as the target for `FromRow` or `FromSql`, and then
    /// call `From::from` to convert it the `self.ty`.
    pub(crate) from: Option<String>,
    /// Override the name of the actual sql column instead of using `self.ident`.
    /// Is not compatible with `flatten` since no column is needed there.
    pub(crate) rename: Option<String>,

    /// Indicates that this field is the primary key of the table.
    pub(crate) primary_key: Option<()>,

    /// Ignore this field for any Sql related operations.
    pub(crate) skip: Option<()>,

    /// Normally, non atomic types are handed to rusqlite as references when
    /// used as params. But some types like Uuid should be handed as values.
    /// This attribute allows to specify that.
    pub(crate) value: Option<()>,
}

impl SqliteField {
    /// Returns the name that maps to the actuall sql column
    /// By default this is the same as the rust field name but can be overwritten by `#[from_row(rename = "..")]`.
    pub(crate) fn column_name(&self) -> String {
        self.rename
            .as_ref()
            .map(Clone::clone)
            .unwrap_or_else(|| self.ident.as_ref().unwrap().to_string())
    }

    pub(crate) fn is_primary_key(&self) -> bool {
        self.primary_key.is_some()
    }
}
