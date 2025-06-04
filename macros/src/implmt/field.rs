use quote::ToTokens;
use syn::Field;

use super::column::ColumnType;

#[derive(Debug, Default)]
pub(super) struct FieldDefinition {
    col_name: String,
    col_type: ColumnType,
    unique: bool,
    default_value: Option<&'static str>,
    length: Option<usize>,
    serial: bool, // autoincrement field
}

impl FieldDefinition {
    pub fn col_name(&self) -> &str {
        &self.col_name
    }

    pub fn col_type(&self) -> &ColumnType {
        &self.col_type
    }
}

impl From<&Field> for FieldDefinition {
    fn from(value: &Field) -> Self {
        let Field { ident, ty, .. } = value;
        let col_name = ident
            .as_ref()
            .map(|v| v.to_token_stream().to_string())
            .unwrap_or("".to_string());

        let col_type = ty.to_token_stream().to_string();

        FieldDefinition {
            col_name,
            col_type: col_type.as_str().into(),
            ..Default::default()
        }
    }
}
