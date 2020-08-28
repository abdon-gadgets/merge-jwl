use anyhow::{bail, ensure, Context, Result};
use rusqlite::{params, Connection, DatabaseName, NO_PARAMS};
use std::rc::Rc;
use tracing::{event, Level};

#[derive(Debug, Clone, Default)]
pub struct Database {
    pub schema_sql: Vec<String>, // TODO: Optimize to do this only once
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
    /// Primary key and rowid.
    pub location_id: u32,
    /// The Bible book number (or null if not Bible).
    pub book_number: Option<u32>,
    /// The Bible chapter number (or null if not Bible).
    pub chapter_number: Option<u32>,
    /// MEPS Document ID.
    pub document_id: Option<u32>,
    /// The track. TODO: Semantics unknown.
    pub track: Option<u32>,
    /// A reference to the publication issue (or 0 if not applicable), e.g. "20171100".
    pub issue_tag_number: u32,
    /// Publication key symbol, or empty string.
    pub key_symbol: Option<Rc<String>>,
    /// MEPS language.
    pub meps_language: u32,
    /// - 0: Standard location entry
    /// - 1: Reference to publication (see Bookmark.PublicationLocationId)
    /// - 2: Unknown
    /// - 3: Unknown
    pub r#type: u32,
    /// Title, like 'Luke 17' or 'Record Your Progress'.
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Note {
    /// Primary key and rowid.
    pub note_id: u32,
    /// A GUID.
    pub guid: String,
    /// UserMark reference if the note is associated with user-highlighting.
    pub user_mark_id: Option<u32>,
    /// Location reference.
    pub location_id: Option<u32>,
    /// User-defined note title.
    pub title: Option<Rc<String>>,
    /// User-defined note content.
    pub content: Option<Rc<String>>,
    /// Time stamp when the note was last edited, ISO 8601 format.
    pub last_modified: Rc<String>,
    /// The type of block associated with the note.
    /// Types:
    /// - 0: The note is associated with the document rather than a block of text within it.
    /// - 1: The note is associated with a paragraph in a publication.
    /// - 2: The note is associated with a verse in the Bible.
    /// See also UserMarkId which may better define the associated block of text.
    pub block_type: u32,
    /// Helps to locate the block of text associated with the note.
    pub block_identifier: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct InputField {
    /// Primary key and rowid.
    pub location_id: u32,
    /// Unknown.
    pub text_tag: String,
    /// Unknown.
    pub value: String,
}

/// Tag with ID 1 has type 0 and name "Favorite"
#[derive(Debug, Clone)]
pub struct Tag {
    /// Primary key and rowid.
    pub tag_id: u32,
    /// Tag type (0 = Favourite, 1 = User-defined, 2 = Unknown).
    pub r#type: u32,
    /// User-defined tag name.
    pub name: String,
    /// Image (added in db version 7 April 2020).
    pub image_filename: Option<String>,
}

/// Many-to-many junction table for tags.
/// Columns PlaylistItemId, LocationId and NoteId are mutually exclusive.
/// Schema changed in version 7.
#[derive(Debug, Clone)]
pub struct TagMap {
    /// Primary key and rowid.
    pub tag_map_id: u32,
    /// Playlist reference.
    pub playlist_item_id: Option<u32>,
    /// Location reference.
    pub location_id: Option<u32>,
    /// Note reference.
    pub note_id: Option<u32>,
    /// Tag reference.
    pub tag_id: u32,
    /// The zero-based position of the tag map entry among all entries having the same TagId.
    /// Tagged items can be ordered in the JWL application.
    pub position: u32,
}

#[derive(Debug, Clone)]
pub struct BlockRange {
    /// Primary key and rowid.
    pub block_range_id: u32,
    /// Block type (1: Publication, 2: Bible)
    pub block_type: u32,
    /// Paragraph or verse ID, one based.
    pub identifier: u32,
    /// Zero-based word in a sentence that marks the start of the highlight.
    pub start_token: Option<u32>,
    /// Zero-based word in a sentence that marks the end of the highlight (inclusive).
    pub end_token: Option<u32>,
    /// UserMark reference.
    pub user_mark_id: u32,
}

#[derive(Debug, Clone)]
pub struct Bookmark {
    /// Primary key and rowid.
    pub bookmark_id: u32,
    /// Location reference.
    pub location_id: u32,
    /// Publication reference, Location.Type = 1.
    pub publication_location_id: u32,
    /// Zero-based order of bookmarks (one of 10 slots with different colors).
    pub slot: u32,
    /// Title.
    pub title: Rc<String>,
    /// Snippet of bookmarked text.
    pub snippet: Option<Rc<String>>,
    /// Block type:
    /// - 0: Bible chapter?
    /// - 1: Publication paragraph
    /// - 2: Bible verse
    pub block_type: u32,
    /// One-based paragraph or verse identifier.
    pub block_identifier: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct UserMark {
    /// Primary key and rowid.
    pub user_mark_id: u32,
    /// The index of the marking (highlight) color.
    pub color_index: u32,
    /// Location reference.
    pub location_id: u32,
    /// Unknown (always 0?).
    pub style_index: u32,
    /// A GUID.
    pub user_mark_guid: String,
    /// Unknown (observed 0 and 1).
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
        let user_version: i64 = conn.query_row("PRAGMA user_version", NO_PARAMS, |r| r.get(0))?;
        ensure!(user_version == 8, "user_version {}", user_version);

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
        let journal_mode: String =
            conn.query_row("PRAGMA journal_mode=off", NO_PARAMS, |r| r.get(0))?;
        ensure!(&journal_mode == "off", "journal_mode {}", &journal_mode);
        conn.execute_batch("BEGIN TRANSACTION")?; // 3x faster
        for sql in &self.schema_sql {
            conn.execute_batch(sql)?;
        }
        conn.execute_batch("PRAGMA user_version=8")?;
        let s = Export {
            conn: &conn,
            database: self,
        };
        s.last_modified()?;
        s.locations()?;
        s.bookmarks()?;
        s.input_fields()?;
        s.user_marks()?;
        s.notes()?;
        s.block_ranges()?;
        // playlist_media
        // playlist_item
        // playlist_item_child
        s.tags()?;
        s.tag_maps()?;
        conn.execute_batch("END TRANSACTION")?;
        // foreign_key_check(&conn)?;
        mem_file[18..20].copy_from_slice(&[2, 2]);
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
            // TODO: Optimize string cache deduplicate
            key_symbol: r.get::<_, Option<String>>(6)?.map(Rc::new),
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
            title: r.get::<_, Option<String>>(4)?.map(Rc::new),
            content: r.get::<_, Option<String>>(5)?.map(Rc::new),
            last_modified: Rc::new(r.get(6)?),
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
            tag_map_id: r.get(0)?,
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
            title: Rc::new(r.get(4)?),
            snippet: r.get::<_, Option<String>>(5)?.map(Rc::new),
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
    fn last_modified(&self) -> Result<()> {
        event!(Level::DEBUG, "write last modified");
        self.conn.execute(
            "INSERT INTO LastModified VALUES (?)",
            params![&self.database.last_modified],
        )?;
        Ok(())
    }

    fn locations(&self) -> Result<()> {
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

    fn notes(&self) -> Result<()> {
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

    fn user_marks(&self) -> Result<()> {
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

    fn tags(&self) -> Result<()> {
        event!(Level::DEBUG, "write tags");
        let mut stmt = self.conn.prepare("INSERT INTO Tag VALUES (?,?,?,?)")?;
        for t in &self.database.tags {
            stmt.execute(params![t.tag_id, t.r#type, t.name, t.image_filename,])?;
        }
        Ok(())
    }

    fn tag_maps(&self) -> Result<()> {
        event!(Level::DEBUG, "write tag_maps");
        let mut stmt = self
            .conn
            .prepare("INSERT INTO TagMap VALUES (?,?,?,?,?,?)")?;
        for t in &self.database.tag_maps {
            stmt.execute(params![
                t.tag_map_id,
                t.playlist_item_id,
                t.location_id,
                t.note_id,
                t.tag_id,
                t.position,
            ])?;
        }
        Ok(())
    }

    fn block_ranges(&self) -> Result<()> {
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

    fn bookmarks(&self) -> Result<()> {
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

    fn input_fields(&self) -> Result<()> {
        event!(Level::DEBUG, "write input_fields");
        let mut stmt = self.conn.prepare("INSERT INTO InputField VALUES (?,?,?)")?;
        for i in &self.database.input_fields {
            stmt.execute(params![i.location_id, i.text_tag, i.value,])?;
        }
        Ok(())
    }
}
