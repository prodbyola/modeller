use quote::ToTokens;
use syn::Field;

use crate::implmt::backend_type::BackendType;

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

    pub fn to_sql(&self, bt: &BackendType) -> String {
        use BackendType::*;
        let col = &self.col_name;

        if self.serial {
            match bt {
                MySql => format!("{col} INT AUTO_INCREMENT PRIMARY KEY"),
                Postgres => format!("{col} INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY"),
                Sqlite => format!("{col} INTEGER PRIMARY KEY AUTOINCREMENT"),
            }
        } else {
            let col_type = &self.col_type.to_sql(&self.length);
            let unique = if self.unique { "UNIQUE" } else { "" };
            let default_value = &self
                .default_value
                .map(|v| format!(" DEFAULT {v}"))
                .unwrap_or(String::new());
            format!("{col} {col_type} {unique} {default_value}")
        }
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
