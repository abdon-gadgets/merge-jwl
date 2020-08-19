use anyhow::{anyhow, ensure, Context, Result};
use rusqlite::{params, Connection, DatabaseName, NO_PARAMS};
use tracing::{event, Level};

#[derive(Debug)]
pub struct Database {
    time_last_modified: String,
    locations: Vec<Location>,
}

#[derive(Debug)]
pub struct Location {
    location_id: u32,
    book_number: Option<u32>,
    chapter_number: Option<u32>,
    document_id: Option<u32>,
    track: Option<u32>,
    issue_tag_number: u32,
    key_symbol: Option<String>,
    meps_language: u32,
    r#type: u32,
    title: Option<String>,
}

impl Database {
    pub fn read(mem_file: Vec<u8>) -> Result<Self> {
        ensure!(mem_file.starts_with(b"SQLite format 3\0"), "Invalid header");
        let conn = Connection::open_in_memory().context("open_in_memory")?;
        conn.query_row("PRAGMA locking_mode=EXCLUSIVE", NO_PARAMS, |_| Ok(()))
            .context("locking_mode EXCLUSIVE")?;
        conn.deserialize(DatabaseName::Main, mem_file)
            .context("Deserialize")?;

        Ok(Database {
            time_last_modified: map_last_modified(&conn)?,
            locations: map_locations(&conn)?,
        })
    }
}

fn map_last_modified(conn: &Connection) -> rusqlite::Result<String> {
    conn.query_row("SELECT LastModified FROM LastModified", NO_PARAMS, |r| {
        r.get(0)
    })
}

fn map_locations(conn: &Connection) -> rusqlite::Result<Vec<Location>> {
    let mut stmt = conn.prepare("SELECT * FROM Location")?;
    let rows = stmt.query_map(NO_PARAMS, |r| {
        Ok(Location {
            location_id: r.get(0)?,
            book_number: r.get(1)?,
            chapter_number: r.get(2)?,
            document_id: r.get(3)?,
            track: r.get(4)?,
            issue_tag_number: r.get(5)?,
            key_symbol: r.get(6)?,
            meps_language: r.get(7)?,
            r#type: r.get(8)?,
            title: r.get(9)?,
        })
    })?;
    rows.collect()
}
