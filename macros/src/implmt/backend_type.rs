pub enum BackendType {
    MySql,
    Postgres,
    Sqlite,
}

impl<'a> From<&'a str> for BackendType {
    fn from(value: &'a str) -> BackendType {
        use BackendType::*;

        if value.starts_with("mysql") {
            MySql
        } else if value.starts_with("postgres") {
            Postgres
        } else {
            Sqlite
        }
    }
}
