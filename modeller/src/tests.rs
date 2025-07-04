use definitions::{
    bincode::{self, config},
    field::FkOnDelete,
    model::ModelDefinition,
};

use crate::prelude::*;

#[derive(Modeller)]
struct MockTable {
    #[modeller(foreign_key(references = "users(id)", on_delete = "cascade"))]
    pub user_id: u64,
}

#[test]
fn test_foreign_key() -> OpResult<()> {
    let stream = MockTable::get_stream();
    let config = config::standard();
    let (model, _): (ModelDefinition, _) = bincode::decode_from_slice(&stream, config)?;

    let field = model.fields().iter().find(|f| f.col_name() == "user_id");
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
