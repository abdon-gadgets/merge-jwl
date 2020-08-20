use crate::{Database, Result};

pub fn merge_databases(mut originals: impl Iterator<Item = Database>) -> Result<Database> {
    // let mut result = Database::default();
    // for mut database in originals {
    //     merge(&database, &mut result);
    // }
    // TODO: PRAGMA foreign_keys=ON
    Ok(originals.next().unwrap())
}

fn merge(src: &Database, dst: &mut Database) {}
