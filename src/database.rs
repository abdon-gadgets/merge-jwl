use anyhow::{anyhow, ensure, Context, Result};
use rusqlite::{params, Connection, DatabaseName};
use tracing::{event, Level};

#[derive(Debug)]
pub struct Database {
    time_last_modified: String,
}

impl Database {
    pub fn read(mem_file: Vec<u8>) -> Result<Self> {
        ensure!(mem_file.starts_with(b"SQLite format 3\0"), "Invalid header");
        let conn = Connection::open_in_memory().context("open_in_memory")?;
        conn.query_row("PRAGMA locking_mode=EXCLUSIVE", params![], |_| Ok(()))
            .context("locking_mode EXCLUSIVE")?;
        conn.deserialize(DatabaseName::Main, mem_file)
            .context("Deserialize")?;

        Ok(Database {
            time_last_modified: read_last_modified(&conn)?,
        })
    }
}

fn read_last_modified(conn: &Connection) -> Result<String> {
    conn.query_row("SELECT LastModified FROM LastModified", params![], |r| {
        r.get(0)
    })
    .context("Query LastModified")
}

// struct DataAccess {
//     conn: Connection,
// }
