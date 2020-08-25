use anyhow::{bail, ensure, Context, Result};
use rusqlite::{params, Connection, DatabaseName, NO_PARAMS};
use std::rc::Rc;
use tracing::{event, Level};

#[derive(Debug, Clone)]
pub struct Database {
    schema_sql: Vec<String>, // TODO: Optimize to do this only once
    pub last_modified: String,
    pub locations: Vec<Location>,
    pub notes: Vec<Note>,
    pub input_fields: Vec<InputField>,
    pub tags: Vec<Tag>,
    pub tag_maps: Vec<TagMap>,
    pub block_ranges: Vec<BlockRange>,
    pub bookmarks: Vec<Bookmark>,
    pub user_marks: Vec<UserMark>,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub location_id: u32,
    pub book_number: Option<u32>,
    pub chapter_number: Option<u32>,
    pub document_id: Option<u32>,
    pub track: Option<u32>,
    pub issue_tag_number: u32,
    pub key_symbol: Option<Rc<String>>,
    pub meps_language: u32,
    pub r#type: u32,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub note_id: u32,
    pub guid: String,
    pub user_mark_id: Option<u32>,
    pub location_id: Option<u32>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub last_modified: String,
    pub block_type: u32,
    pub block_identifier: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct InputField {
    pub location_id: u32,
    pub text_tag: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub tag_id: u32,
    pub r#type: u32,
    pub name: String,
    pub image_filename: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TagMap {
    pub tag_read_id: u32,
    pub playlist_item_id: Option<u32>,
    pub location_id: Option<u32>,
    pub note_id: Option<u32>,
    pub tag_id: u32,
    pub position: u32,
}

#[derive(Debug, Clone)]
pub struct BlockRange {
    pub block_range_id: u32,
    pub block_type: u32,
    pub identifier: u32,
    pub start_token: Option<u32>,
    pub end_token: Option<u32>,
    pub user_mark_id: u32,
}

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub bookmark_id: u32,
    pub location_id: u32,
    pub publication_location_id: u32,
    pub slot: u32,
    pub title: String,
    pub snippet: Option<String>,
    pub block_type: u32,
    pub block_identifier: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct UserMark {
    pub user_mark_id: u32,
    pub color_index: u32,
    pub location_id: u32,
    pub style_index: u32,
    pub user_mark_guid: String,
    pub version: u32,
}

#[derive(Debug)]
struct Violation {
    table: String,
    row_id: Option<i64>,
    parent: String,
    f_kid: i64,
}

impl Database {
    pub fn read(mut mem_file: Vec<u8>) -> Result<Self> {
        ensure!(mem_file.starts_with(b"SQLite format 3\0"), "Invalid header");
        // Set file format read/write version numbers to 1 for journal mode rollback
        let file_format_version = &mut mem_file[18..20];
        match file_format_version {
            [1, 1] => (),
            [2, 2] => file_format_version.copy_from_slice(&[1, 1]),
            _ => bail!("Unknown file format read/write version"),
        }
        let conn = Connection::open_in_memory().context("open_in_memory")?;
        conn.deserialize(DatabaseName::Main, mem_file)
            .context("Deserialize")?;
        let journal_mode: String =
            conn.query_row("PRAGMA journal_mode", NO_PARAMS, |r| r.get(0))?;
        ensure!(&journal_mode == "memory", "journal_mode {}", &journal_mode);

        // Replaces FixupAnomalies
        foreign_key_check(&conn)?;

        read_playlist_media(&conn)?;

        Ok(Database {
            schema_sql: read_schema(&conn)?,
            last_modified: read_last_modified(&conn)?,
            locations: read_locations(&conn)?,
            notes: read_notes(&conn)?,
            input_fields: read_input_fields(&conn)?,
            tags: read_tags(&conn)?,
            tag_maps: read_tag_maps(&conn)?,
            block_ranges: read_block_ranges(&conn)?,
            bookmarks: read_bookmarks(&conn)?,
            user_marks: read_user_marks(&conn)?,
        })
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut mem_file = Vec::new(); // TODO: Guess size
        let conn = Connection::open_in_memory()?.into_borrowing();
        conn.deserialize_writable(DatabaseName::Main, &mut mem_file)?;
        // conn.execute_batch("PRAGMA foreign_keys=0")?;
        for sql in &self.schema_sql {
            conn.execute_batch(sql)?;
        }
        let s = Export {
            conn: &conn,
            database: self,
        };
        s.locations()?;
        s.bookmarks()?;
        s.user_marks()?;
        s.notes()?;
        s.block_ranges()?;
        // playlist_media
        // playlist_item
        // playlist_item_child
        s.tags()?;
        s.tag_maps()?;
        s.input_fields()?;
        // foreign_key_check(&conn)?;
        Ok(mem_file)
    }
}

fn foreign_key_check(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT * FROM pragma_foreign_key_check")?;
    let violations = stmt.query_map(NO_PARAMS, |r| {
        Ok(Violation {
            table: r.get(0)?,
            row_id: r.get(1)?,
            parent: r.get(2)?,
            f_kid: r.get(3)?,
        })
    })?;
    let violations = violations.collect::<rusqlite::Result<Vec<_>>>()?;
    if !violations.is_empty() {
        bail!("Foreign key check failed: {:?}", violations);
    }
    Ok(())
}

fn read_schema(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT sql FROM sqlite_master WHERE sql IS NOT NULL")?;
    let rows = stmt.query_map(NO_PARAMS, |r| r.get::<_, String>(0))?;
    rows.collect()
}

fn read_last_modified(conn: &Connection) -> rusqlite::Result<String> {
    conn.query_row("SELECT LastModified FROM LastModified", NO_PARAMS, |r| {
        r.get(0)
    })
}

fn read_locations(conn: &Connection) -> rusqlite::Result<Vec<Location>> {
    let mut stmt = conn.prepare("SELECT * FROM Location")?;
    let rows = stmt.query_map(NO_PARAMS, |r| {
        Ok(Location {
            location_id: r.get(0)?,
            book_number: r.get(1)?,
            chapter_number: r.get(2)?,
            document_id: r.get(3)?,
            track: r.get(4)?,
            issue_tag_number: r.get(5)?,
            // TODO: Optimize string pool deduplicate
            key_symbol: r.get::<_, Option<String>>(6)?.map(|k| Rc::new(k)),
            meps_language: r.get(7)?,
            r#type: r.get(8)?,
            title: r.get(9)?,
        })
    })?;
    rows.collect()
}

fn read_notes(conn: &Connection) -> rusqlite::Result<Vec<Note>> {
    let mut stmt = conn.prepare("SELECT * FROM Note")?;
    let rows = stmt.query_map(NO_PARAMS, |r| {
        Ok(Note {
            note_id: r.get(0)?,
            guid: r.get(1)?,
            user_mark_id: r.get(2)?,
            location_id: r.get(3)?,
            title: r.get(4)?,
            content: r.get(5)?,
            last_modified: r.get(6)?,
            block_type: r.get(7)?,
            block_identifier: r.get(8)?,
        })
    })?;
    rows.collect()
}

fn read_input_fields(conn: &Connection) -> rusqlite::Result<Vec<InputField>> {
    let mut stmt = conn.prepare("SELECT * FROM InputField")?;
    let rows = stmt.query_map(NO_PARAMS, |r| {
        Ok(InputField {
            location_id: r.get(0)?,
            text_tag: r.get(1)?,
            value: r.get(2)?,
        })
    })?;
    rows.collect()
}

fn read_tags(conn: &Connection) -> rusqlite::Result<Vec<Tag>> {
    let mut stmt = conn.prepare("SELECT * FROM Tag")?;
    let rows = stmt.query_map(NO_PARAMS, |r| {
        Ok(Tag {
            tag_id: r.get(0)?,
            r#type: r.get(1)?,
            name: r.get(2)?,
            image_filename: r.get(3)?,
        })
    })?;
    rows.collect()
}

fn read_tag_maps(conn: &Connection) -> rusqlite::Result<Vec<TagMap>> {
    let mut stmt = conn.prepare("SELECT * FROM TagMap")?;
    let rows = stmt.query_map(NO_PARAMS, |r| {
        Ok(TagMap {
            tag_read_id: r.get(0)?,
            playlist_item_id: r.get(1)?,
            location_id: r.get(2)?,
            note_id: r.get(3)?,
            tag_id: r.get(4)?,
            position: r.get(5)?,
        })
    })?;
    rows.collect()
}

fn read_block_ranges(conn: &Connection) -> rusqlite::Result<Vec<BlockRange>> {
    let mut stmt = conn.prepare("SELECT * FROM BlockRange")?;
    let rows = stmt.query_map(NO_PARAMS, |r| {
        Ok(BlockRange {
            block_range_id: r.get(0)?,
            block_type: r.get(1)?,
            identifier: r.get(2)?,
            start_token: r.get(3)?,
            end_token: r.get(4)?,
            user_mark_id: r.get(5)?,
        })
    })?;
    rows.collect()
}

fn read_bookmarks(conn: &Connection) -> rusqlite::Result<Vec<Bookmark>> {
    // TODO: ORDER BY Slot? "ensure bookmarks appear in similar order to original"
    let mut stmt = conn.prepare("SELECT * FROM Bookmark")?;
    let rows = stmt.query_map(NO_PARAMS, |r| {
        Ok(Bookmark {
            bookmark_id: r.get(0)?,
            location_id: r.get(1)?,
            publication_location_id: r.get(2)?,
            slot: r.get(3)?,
            title: r.get(4)?,
            snippet: r.get(5)?,
            block_type: r.get(6)?,
            block_identifier: r.get(7)?,
        })
    })?;
    rows.collect()
}

fn read_user_marks(conn: &Connection) -> rusqlite::Result<Vec<UserMark>> {
    let mut stmt = conn.prepare("SELECT * FROM UserMark")?;
    let rows = stmt.query_map(NO_PARAMS, |r| {
        Ok(UserMark {
            user_mark_id: r.get(0)?,
            color_index: r.get(1)?,
            location_id: r.get(2)?,
            style_index: r.get(3)?,
            user_mark_guid: r.get(4)?,
            version: r.get(5)?,
        })
    })?;
    rows.collect()
}

fn read_playlist_media(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT * FROM PlaylistMedia")?;
    ensure!(
        stmt.query(NO_PARAMS)?.next()?.is_none(),
        "PlaylistMedia not yet implemented"
    );
    // TODO: Merge PlaylistMedia, PlaylistItem, PlaylistItemChild
    Ok(())
}

struct Export<'a> {
    conn: &'a Connection,
    database: &'a Database,
}

impl Export<'_> {
    fn locations(&self) -> rusqlite::Result<()> {
        event!(Level::DEBUG, "write locations");
        let mut stmt = self
            .conn
            .prepare("INSERT INTO Location VALUES (?,?,?,?,?,?,?,?,?,?)")?;
        for l in &self.database.locations {
            stmt.execute(params![
                l.location_id,
                l.book_number,
                l.chapter_number,
                l.document_id,
                l.track,
                l.issue_tag_number,
                l.key_symbol,
                l.meps_language,
                l.r#type,
                l.title,
            ])?;
        }
        Ok(())
    }

    fn notes(&self) -> rusqlite::Result<()> {
        event!(Level::DEBUG, "write notes");
        let mut stmt = self
            .conn
            .prepare("INSERT INTO Note VALUES (?,?,?,?,?,?,?,?,?)")?;
        for n in &self.database.notes {
            stmt.execute(params![
                n.note_id,
                n.guid,
                n.user_mark_id,
                n.location_id,
                n.title,
                n.content,
                n.last_modified,
                n.block_type,
                n.block_identifier,
            ])?;
        }
        Ok(())
    }

    fn user_marks(&self) -> rusqlite::Result<()> {
        event!(Level::DEBUG, "write user_marks");
        let mut stmt = self
            .conn
            .prepare("INSERT INTO UserMark VALUES (?,?,?,?,?,?)")?;
        for u in &self.database.user_marks {
            stmt.execute(params![
                u.user_mark_id,
                u.color_index,
                u.location_id,
                u.style_index,
                u.user_mark_guid,
                u.version,
            ])?;
        }
        Ok(())
    }

    fn tags(&self) -> rusqlite::Result<()> {
        event!(Level::DEBUG, "write tags");
        let mut stmt = self.conn.prepare("INSERT INTO Tag VALUES (?,?,?,?)")?;
        for t in &self.database.tags {
            stmt.execute(params![t.tag_id, t.r#type, t.name, t.image_filename,])?;
        }
        Ok(())
    }

    fn tag_maps(&self) -> rusqlite::Result<()> {
        event!(Level::DEBUG, "write tag_maps");
        let mut stmt = self
            .conn
            .prepare("INSERT INTO TagMap VALUES (?,?,?,?,?,?)")?;
        for t in &self.database.tag_maps {
            stmt.execute(params![
                t.tag_read_id,
                t.playlist_item_id,
                t.location_id,
                t.note_id,
                t.tag_id,
                t.position,
            ])?;
        }
        Ok(())
    }

    fn block_ranges(&self) -> rusqlite::Result<()> {
        event!(Level::DEBUG, "write block_ranges");
        let mut stmt = self
            .conn
            .prepare("INSERT INTO BlockRange VALUES (?,?,?,?,?,?)")?;
        for b in &self.database.block_ranges {
            stmt.execute(params![
                b.block_range_id,
                b.block_type,
                b.identifier,
                b.start_token,
                b.end_token,
                b.user_mark_id,
            ])?;
        }
        Ok(())
    }

    fn bookmarks(&self) -> rusqlite::Result<()> {
        event!(Level::DEBUG, "write bookmarks");
        let mut stmt = self
            .conn
            .prepare("INSERT INTO Bookmark VALUES (?,?,?,?,?,?,?,?)")?;
        for b in &self.database.bookmarks {
            stmt.execute(params![
                b.bookmark_id,
                b.location_id,
                b.publication_location_id,
                b.slot,
                b.title,
                b.snippet,
                b.block_type,
                b.block_identifier,
            ])?;
        }
        Ok(())
    }

    fn input_fields(&self) -> rusqlite::Result<()> {
        event!(Level::DEBUG, "write input_fields");
        let mut stmt = self.conn.prepare("INSERT INTO InputField VALUES (?,?,?)")?;
        for i in &self.database.input_fields {
            stmt.execute(params![i.location_id, i.text_tag, i.value,])?;
        }
        Ok(())
    }
}
