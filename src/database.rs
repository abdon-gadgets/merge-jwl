use anyhow::{anyhow, ensure, Context, Result};
use rusqlite::{params, Connection, DatabaseName, NO_PARAMS};
use tracing::{event, Level};

#[derive(Debug)]
pub struct Database {
    last_modified: String,
    locations: Vec<Location>,
    notes: Vec<Note>,
    input_fields: Vec<InputField>,
    tags: Vec<Tag>,
    tag_maps: Vec<TagMap>,
    block_ranges: Vec<BlockRange>,
    bookmarks: Vec<Bookmark>,
    user_marks: Vec<UserMark>,
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

#[derive(Debug)]
pub struct Note {
    note_id: u32,
    guid: String,
    user_mark_id: Option<u32>,
    location_id: Option<u32>,
    title: Option<String>,
    content: Option<String>,
    last_modified: String,
    block_type: u32,
    block_identifier: Option<u32>,
}

#[derive(Debug)]
pub struct InputField {
    location_id: u32,
    text_tag: String,
    value: String,
}

#[derive(Debug)]
pub struct Tag {
    tag_id: u32,
    r#type: u32,
    name: String,
    image_filename: Option<String>,
}

#[derive(Debug)]
pub struct TagMap {
    tag_read_id: u32,
    playlist_item_id: Option<u32>,
    location_id: Option<u32>,
    note_id: Option<u32>,
    tag_id: u32,
    position: u32,
}

#[derive(Debug)]
pub struct BlockRange {
    block_range_id: u32,
    block_type: u32,
    identifier: u32,
    start_token: Option<u32>,
    end_token: Option<u32>,
    user_mark_id: u32,
}

#[derive(Debug)]
pub struct Bookmark {
    bookmark_id: u32,
    location_id: u32,
    publication_location_id: u32,
    slot: u32,
    title: String,
    snippet: Option<String>,
    block_type: u32,
    block_identifier: Option<u32>,
}

#[derive(Debug)]
pub struct UserMark {
    user_mark_id: u32,
    color_index: u32,
    location_id: u32,
    style_index: u32,
    user_mark_guid: String,
    version: u32,
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
            key_symbol: r.get(6)?,
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
    let mut stmt = conn.prepare("SELECT * FROM Bookmark ORDER BY Slot")?;
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
