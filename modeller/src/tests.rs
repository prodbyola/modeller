use definitions::{
    bincode::{self, config},
    field::{FieldDefinition, FkOnDelete},
    model::ModelDefinition,
};

use crate::prelude::*;

#[derive(Modeller)]
#[modeller(table_name = "users")]
struct User {
    pub id: u64,
}

#[derive(Modeller)]
#[modeller(unique_together(user_id, project_id))]
struct MockTable {
    #[modeller(foreign_key(rf = "users(id)", on_delete = "cascade"))]
    pub user_id: u64,
    pub project_id: u64,
}

#[test]
fn test_table_name() -> OpResult<()> {
    let mut model = MockTable::get_definition()?;
    assert_eq!(model.name(), "mock_table");

    model = User::get_definition()?;
    assert_eq!(model.name(), "users");

    Ok(())
}

#[test]
fn test_unique_together() -> OpResult<()> {
    let mut model = MockTable::get_definition()?;
    let mut ut = model.unique_together();

    assert!(ut.is_some());
    if let Some(ut) = ut {
        assert_eq!(ut.len(), 2)
    }

    model = User::get_definition()?;
    ut = model.unique_together();
    assert!(ut.is_none());

    Ok(())
}

#[test]
fn test_foreign_key() -> OpResult<()> {
    let model = MockTable::get_definition()?;
    let field = get_field(&model, "user_id");

    if let Some(field) = field {
        let fk = field.foreign_key();
        assert!(field.foreign_key().is_some());

        if let Some(fk) = fk {
            assert_eq!(fk.references(), "users(id)");
            assert_eq!(fk.on_delete(), &FkOnDelete::Cascade);

            assert_eq!(
                fk.to_sql("user_id"),
                "FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE"
            );
        }
    }

    Ok(())
}

fn get_field<'a>(model: &'a ModelDefinition, col_name: &str) -> Option<&'a FieldDefinition> {
    model.fields().iter().find(|f| f.col_name() == col_name)
}
