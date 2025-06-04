#[derive(Debug, Default)]
pub(super) enum ColumnType {
    Int8,
    Int16,
    Int32,
    Int64,
    Text,
    #[default]
    VarChar,
    Nullable(Box<ColumnType>),
}

impl ColumnType {
    pub fn to_sql(&self, len: &Option<usize>) -> String {
        use ColumnType::*;

        let len_str = len.map(|v| format!("({v})")).unwrap_or(String::new());

        let mut sql = match self {
            VarChar => format!("{}{len_str}", self.to_str()),
            _ => format!("{}", self.to_str()),
        };

        match self {
            Nullable(_) => sql,
            _ => format!("{sql} NOT NULL"),
        }
    }

    pub fn to_str(&self) -> &'static str {
        use ColumnType::*;

        match self {
            Int8 => "BIT",
            Int16 => "SMALLINT",
            Int32 => "INTEGER",
            Int64 => "BIGINT",
            Text => "TEXT",
            VarChar => "VARCHAR",
            Nullable(inner) => inner.to_str(),
        }
    }
}

impl<'a> From<&'a str> for ColumnType {
    fn from(ty: &'a str) -> Self {
        use ColumnType::*;

        if ty.starts_with("Option") {
            let rem_opt = ty.trim_start_matches("Option < ");
            let trimmed = rem_opt.trim_end_matches(" >");

            let inner = Box::new(trimmed.into());
            Nullable(inner)
        } else {
            match ty {
                "u64" | "i64" => Int64,
                "u32" | "i32" => Int32,
                "u16" | "i16" => Int16,
                "u8" | "i8" => Int8,
                "String" | "str" => VarChar,
                "Text" => Text,
                _ => panic!("ColumnDefinition not implemented for {ty}"),
            }
        }
    }
}
