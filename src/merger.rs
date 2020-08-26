use crate::database::{BlockRange, Location, TagMap};
use crate::{anyhow, bail, Database, Result};
use chrono::DateTime;
use std::collections::{HashMap, HashSet};
use std::iter;
use std::rc::Rc;
use tracing::{event, Level};

pub fn merge_databases(mut originals: impl Iterator<Item = Database>) -> Result<Database> {
    let mut result = originals.next().ok_or_else(|| anyhow!("At least 1 db"))?;
    for mut database in originals {
        event!(Level::DEBUG, "Merge databases");
        merge(&mut database, &mut result)?;
    }
    Ok(result)
}

fn merge(src: &mut Database, dst: &mut Database) -> Result<()> {
    let mut s = Merge::new(src, dst);
    s.merge_bookmarks()?;
    s.merge_user_marks()?;
    s.merge_notes()?;
    s.merge_block_ranges()?;
    s.merge_tags()?;
    s.merge_tag_maps()?;
    s.merge_input_field()?;
    Ok(())
}

struct Merge<'a> {
    src: &'a mut Database,
    dst: &'a mut Database,
    user_mark_translate: HashMap<u32, u32>,
    note_translate: HashMap<u32, u32>,
    tag_translate: HashMap<u32, u32>,
    location: LocationMerge,
}

struct LocationMerge {
    location_translate: HashMap<u32, u32>,
    src_location_index: HashMap<u32, Location>,
    dst_location_value_index: HashMap<LocationValue, u32>,
    location_max_id: u32,
}

#[derive(Eq, PartialEq, Hash)]
struct LocationValue {
    book_number: Option<u32>,
    chapter_number: Option<u32>,
    document_id: Option<u32>,
    track: Option<u32>,
    issue_tag_number: u32,
    key_symbol: Option<Rc<String>>,
    meps_language: u32,
    r#type: u32,
}

impl LocationValue {
    fn from(location: &Location) -> Self {
        LocationValue {
            book_number: location.book_number,
            chapter_number: location.chapter_number,
            document_id: location.document_id,
            track: location.track,
            issue_tag_number: location.issue_tag_number,
            key_symbol: location.key_symbol.clone(),
            meps_language: location.meps_language,
            r#type: location.r#type,
        }
    }
}

impl<'a> Merge<'a> {
    fn new(src: &'a mut Database, dst: &'a mut Database) -> Self {
        Merge {
            user_mark_translate: HashMap::new(),
            note_translate: HashMap::new(),
            tag_translate: HashMap::new(),
            location: LocationMerge {
                location_translate: HashMap::new(),
                // TODO: Lazy initialization of indices
                src_location_index: src
                    .locations
                    .drain(..)
                    .map(|l| (l.location_id, l))
                    .collect(),
                dst_location_value_index: dst
                    .locations
                    .iter()
                    .map(|l| (LocationValue::from(l), l.location_id))
                    .collect(),
                // TODO: Optimize first/last
                location_max_id: dst
                    .locations
                    .iter()
                    .map(|l| l.location_id)
                    .max()
                    .unwrap_or(0),
            },
            src,
            dst,
        }
    }

    fn merge_bookmarks(&mut self) -> Result<()> {
        if self.src.bookmarks.is_empty() {
            return Ok(());
        }
        let mut max_id = self
            .dst
            .bookmarks
            .iter()
            .map(|b| b.bookmark_id)
            .max()
            .unwrap_or(0);
        for mut src in self.src.bookmarks.drain(..) {
            src.location_id = self
                .location
                .get_or_insert_location(&mut self.dst.locations, src.location_id);
            src.publication_location_id = self
                .location
                .get_or_insert_location(&mut self.dst.locations, src.publication_location_id);
            // TODO: Understand `existingBookmark = destination.FindBookmark`
            max_id += 1;
            src.bookmark_id = max_id;
            // If the slot was taken, find the first empty slot, max 9
            // TODO: Optimize slot with hashing/indexing
            let mut slots: Vec<_> = self
                .dst
                .bookmarks
                .iter()
                .filter(|b| b.publication_location_id == src.publication_location_id)
                .map(|b| b.slot)
                .chain(iter::once(10)) // TODO: What happens beyond 9
                .collect();
            if slots.contains(&src.slot) {
                slots.sort();
                src.slot = slots
                    .into_iter()
                    .enumerate()
                    .map(|(i, b)| (i as u32, b))
                    .find(|(i, b)| b != i)
                    .map(|i| i.0)
                    .ok_or_else(|| anyhow!("All bookmark slots are filled"))?;
            }
            self.dst.bookmarks.push(src);
        }
        Ok(())
    }

    fn merge_user_marks(&mut self) -> Result<()> {
        if self.src.user_marks.is_empty() {
            return Ok(());
        }
        let guid_map = self
            .dst
            .user_marks
            .iter()
            .map(|u| Ok((parse_guid(&u.user_mark_guid)?, u.user_mark_id)))
            .collect::<Result<HashMap<_, _>>>()?;
        // TODO: Optimize first/last
        let mut user_mark_max_id = self
            .dst
            .user_marks
            .iter()
            .map(|u| u.user_mark_id)
            .max()
            .unwrap_or(0);

        for mut src in self.src.user_marks.drain(..) {
            if let Some(&existing) = guid_map.get(&parse_guid(&src.user_mark_guid)?) {
                assert!(
                    self.user_mark_translate
                        .insert(src.user_mark_id, existing)
                        .is_none(),
                    "Primary key UserMark violated"
                );
            } else {
                let src_id = src.user_mark_id;
                src.location_id = self
                    .location
                    .insert_location(&mut self.dst.locations, src.location_id);
                user_mark_max_id += 1;
                src.user_mark_id = user_mark_max_id;
                self.user_mark_translate.insert(src_id, src.user_mark_id);
                self.dst.user_marks.push(src);
            }
        }
        Ok(())
    }

    fn merge_notes(&mut self) -> Result<()> {
        if self.src.notes.is_empty() {
            return Ok(());
        }
        let mut new_notes = Vec::new();
        let notes = &mut self.dst.notes;
        let mut max_note_id = notes.iter().map(|n| n.note_id).max().unwrap_or(0);
        let mut guid_map = notes
            .iter_mut()
            .map(|n| Ok((parse_guid(&n.guid)?, n)))
            .collect::<Result<HashMap<_, _>>>()?;
        for mut src in self.src.notes.drain(..) {
            if let Some(dst) = guid_map.get_mut(&parse_guid(&src.guid)?) {
                let src_time = DateTime::parse_from_rfc3339(&src.last_modified)?;
                let dst_time = DateTime::parse_from_rfc3339(&dst.last_modified)?;
                if dst_time < src_time {
                    // note already exists in destination, but it's older
                    dst.title = src.title.clone();
                    dst.content = src.content.clone();
                    dst.last_modified = src.last_modified.clone();
                }
                self.note_translate.insert(src.note_id, dst.note_id);
            } else {
                // insert note
                max_note_id += 1;
                let new_id = max_note_id;
                self.note_translate.insert(src.note_id, new_id);
                src.note_id = new_id;
                if let Some(user_mark_id) = src.user_mark_id {
                    src.user_mark_id = Some(
                        self.user_mark_translate
                            .get(&user_mark_id)
                            .copied()
                            .expect("Foreign key Note UserMark violated"),
                    );
                }
                if let Some(location_id) = src.location_id {
                    src.location_id = Some(
                        self.location
                            .get_or_insert_location(&mut self.dst.locations, location_id),
                    );
                }
                new_notes.push(src);
            }
        }
        notes.extend(new_notes);
        Ok(())
    }

    fn merge_block_ranges(&mut self) -> Result<()> {
        if self.src.block_ranges.is_empty() {
            return Ok(());
        }
        let mut max_id = self
            .dst
            .block_ranges
            .iter()
            .map(|b| b.block_range_id)
            .max()
            .unwrap_or(0);
        let mut group_by_user_mark = HashMap::with_capacity(self.dst.user_marks.len());
        for mut src in self.src.block_ranges.drain(..) {
            let user_mark_id = *self
                .user_mark_translate
                .get(&src.user_mark_id)
                .expect("Foreign key BlockRange UserMark violated");
            let existing = group_by_user_mark
                .entry(user_mark_id)
                .or_insert_with(Vec::new);
            if existing.iter().any(|b| block_ranges_overlap(b, &src)) {
                event!(Level::DEBUG, "Remove overlapping BlockRange {:?}", &src);
            } else {
                existing.push(src.clone()); // this clone is cheap
                max_id += 1;
                src.block_range_id = max_id;
                src.user_mark_id = user_mark_id;
                self.dst.block_ranges.push(src);
            }
        }
        Ok(())
    }

    fn merge_tags(&mut self) -> Result<()> {
        if self.src.tags.is_empty() {
            return Ok(());
        }
        let index: HashMap<_, _> = self
            .dst
            .tags
            .iter()
            .map(|t| ((t.r#type, &t.name), t.tag_id))
            .collect();
        // TODO: Optimize max_id loops twice
        let mut max_id = self.dst.tags.iter().map(|t| t.tag_id).max().unwrap_or(0);
        let mut new_tags = Vec::with_capacity(self.src.tags.len());
        for mut src in self.src.tags.drain(..) {
            if let Some(existing) = index.get(&(src.r#type, &src.name)) {
                self.tag_translate.insert(src.tag_id, *existing);
            } else {
                max_id += 1;
                self.tag_translate.insert(src.tag_id, max_id);
                src.tag_id = max_id;
                new_tags.push(src);
            }
        }
        self.dst.tags.extend(new_tags);
        Ok(())
    }

    fn merge_tag_maps(&mut self) -> Result<()> {
        if self.src.tag_maps.is_empty() {
            return Ok(());
        }
        let mut max_id = self
            .dst
            .tag_maps
            .iter()
            .map(|t| t.tag_map_id)
            .max()
            .unwrap_or(0);
        let location_index: HashSet<_> = self
            .dst
            .tag_maps
            .iter()
            .filter_map(|t| Some((t.tag_id, t.location_id?)))
            .collect();
        let note_index: HashSet<_> = self
            .dst
            .tag_maps
            .iter()
            .filter_map(|t| Some((t.tag_id, t.note_id?)))
            .collect();
        for mut src in self.src.tag_maps.drain(..) {
            let tag_id = *self
                .tag_translate
                .get(&src.tag_id)
                .expect("Foreign key TagMap Tag violated");
            src.tag_id = tag_id;
            if let Some(location_id) = src.location_id {
                let location_id = self
                    .location
                    .get_or_insert_location(&mut self.dst.locations, location_id);
                // TODO: It is not known if location was equivalent translation
                if !location_index.contains(&(tag_id, location_id)) {
                    src.location_id = Some(location_id);
                    insert_tag_map(self.dst, &mut max_id, src);
                }
            } else if let Some(note_id) = src.note_id {
                let note_id = *self
                    .note_translate
                    .get(&note_id)
                    .expect("Foreign key TagMap Note violated");
                if !note_index.contains(&(tag_id, note_id)) {
                    src.note_id = Some(note_id);
                    insert_tag_map(self.dst, &mut max_id, src);
                }
            } else if let Some(_playlist_item_id) = src.playlist_item_id {
                // TODO: Implement playlist
            } else {
                panic!("Check constraint TagMap violated");
            }
        }
        self.normalize_tag_map_positions();
        Ok(())
    }

    fn normalize_tag_map_positions(&mut self) {
        // unique constraint on TagId, Position
        let mut tag_groups = HashMap::with_capacity(self.dst.tags.len());
        self.dst.tag_maps.sort_by_key(|t| t.position);
        for tag_map in &mut self.dst.tag_maps {
            let position = tag_groups.entry(tag_map.tag_id).or_insert(0);
            tag_map.position = *position;
            *position += 1;
        }
    }

    fn merge_input_field(&mut self) -> Result<()> {
        if self.src.input_fields.is_empty() {
            return Ok(());
        }
        bail!("InputField merge not yet implemented");
    }
}

impl LocationMerge {
    fn insert_location(&mut self, dst: &mut Vec<Location>, location_id: u32) -> u32 {
        if let Some(&translation) = self.location_translate.get(&location_id) {
            translation
        } else {
            let mut location = self
                .src_location_index
                .remove(&location_id)
                .expect("Foreign key LocationId violated");
            if let Some(&equivalent) = self
                .dst_location_value_index
                .get(&LocationValue::from(&location))
            {
                self.location_translate.insert(location_id, equivalent);
                equivalent
            } else {
                self.location_max_id += 1;
                let new_id = self.location_max_id;
                location.location_id = new_id;
                self.location_translate.insert(location_id, new_id);
                dst.push(location);
                new_id
            }
        }
    }

    fn get_or_insert_location(&mut self, dst: &mut Vec<Location>, id: u32) -> u32 {
        self.location_translate
            .get(&id)
            .copied()
            .unwrap_or_else(|| self.insert_location(dst, id))
    }
}

fn block_ranges_overlap(a: &BlockRange, b: &BlockRange) -> bool {
    if a.start_token == b.start_token && a.end_token == b.end_token {
        true
    } else if a.start_token.is_none()
        || a.end_token.is_none()
        || b.start_token.is_none()
        || b.end_token.is_none()
    {
        false
    } else {
        b.start_token < a.end_token && b.end_token > a.start_token
    }
}

fn insert_tag_map(dst: &mut Database, max_id: &mut u32, mut src: TagMap) {
    *max_id += 1;
    src.tag_map_id = *max_id;
    dst.tag_maps.push(src);
}

fn parse_guid(input: &str) -> Result<u128> {
    if input.len() != 36 {
        bail!("GUID has invalid length {}", input);
    }
    let dash = |i: usize| input.as_bytes()[i] != b'-';
    if dash(8) || dash(13) || dash(18) || dash(23) {
        bail!("GUID has invalid separators {}", input);
    }
    let low = u32::from_str_radix(&input[..8], 16)?; // 4 bytes
    let mid = u16::from_str_radix(&input[9..13], 16)?; // 2 bytes
    let hi = u16::from_str_radix(&input[14..18], 16)?; // 2 bytes
    let seq = u16::from_str_radix(&input[19..23], 16)?; // 2 bytes
    let node = u128::from_str_radix(&input[24..36], 16)?; // 6 bytes
    Ok(node | (seq as u128) << 48 | (hi as u128) << 64 | (mid as u128) << 80 | (low as u128) << 96)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::database::Bookmark;

    #[test]
    fn test_parse_guid() {
        let r = parse_guid("c88af989-da73-4745-bccc-8476f9950a3c").unwrap();
        dbg!(format!("{:x}", r));
        assert_eq!(r, 0xc88af989_da73_4745_bccc_8476f9950a3c);
    }

    #[test]
    fn test_merge_bookmarks() -> Result<()> {
        fn with_slot(slot: u32) -> Bookmark {
            Bookmark {
                bookmark_id: 123,
                location_id: 123,
                publication_location_id: 123,
                slot,
                title: "foo".to_string(),
                snippet: Some("bar".to_string()),
                block_type: 1,
                block_identifier: Some(18),
            }
        }
        fn some_locations() -> Vec<Location> {
            vec![Location {
                location_id: 123,
                book_number: None,
                chapter_number: None,
                document_id: None,
                track: None,
                issue_tag_number: 123,
                key_symbol: None,
                meps_language: 123,
                r#type: 123,
                title: None,
            }]
        }
        fn merge_slots(src: &[u32], dst: &[u32]) -> Result<Vec<u32>> {
            let mut src = Database {
                locations: some_locations(),
                bookmarks: src.iter().copied().map(with_slot).collect(),
                ..Default::default()
            };
            let mut dst = Database {
                locations: some_locations(),
                bookmarks: dst.iter().copied().map(with_slot).collect(),
                ..Default::default()
            };
            let mut state = Merge::new(&mut src, &mut dst);
            state.merge_bookmarks()?;
            let mut vec: Vec<u32> = dst.bookmarks.iter().map(|b| b.slot).collect();
            vec.sort();
            Ok(vec)
        }
        assert_eq!(&merge_slots(&[3, 4], &[1])?, &[1, 3, 4]);
        assert_eq!(&merge_slots(&[0, 1], &[3])?, &[0, 1, 3]);
        assert_eq!(&merge_slots(&[8], &[8])?, &[0, 8]);
        assert_eq!(&merge_slots(&[3], &[0, 3])?, &[0, 1, 3]);
        assert_eq!(&merge_slots(&[0], &[0, 1, 2, 8, 9])?, &[0, 1, 2, 3, 8, 9]);
        assert_eq!(&merge_slots(&[0, 3], &[0, 2, 4, 6])?, &[0, 1, 2, 3, 4, 6]);
        assert_eq!(
            &merge_slots(&[1, 3], &[0, 1, 2, 3, 4, 5, 6, 7])?,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
        );
        merge_slots(&[1, 3], &[0, 1, 2, 3, 4, 5, 6, 7, 8]).unwrap_err();
        merge_slots(&[7], &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap_err();
        Ok(())
    }
}
