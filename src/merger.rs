use crate::database::Location;
use crate::{anyhow, bail, Database, Result};
use std::collections::HashMap;
use tracing::{event, Level};

pub fn merge_databases(mut originals: impl Iterator<Item = Database>) -> Result<Database> {
    let mut result = originals.next().ok_or_else(|| anyhow!("At least 1 db"))?;
    for database in originals {
        event!(Level::DEBUG, "Merge databases");
        merge(&database, &mut result)?;
    }
    Ok(result)
}

fn merge(src: &Database, dst: &mut Database) -> Result<()> {
    let mut s = State {
        user_mark_translate: HashMap::new(),
        location_translate: HashMap::new(),
        // TODO: Lazy initialization of indices
        src_location_index: src.locations.iter().map(|l| (l.location_id, l)).collect(),
        dst_location_value_index: dst
            .locations
            .iter()
            .map(|l| (l.into(), l.location_id))
            .collect(),
        // TODO: Optimize first/last
        location_max_id: dst
            .locations
            .iter()
            .map(|l| l.location_id)
            .max()
            .unwrap_or(0),
        src,
        dst,
    };
    s.merge_user_marks()?;
    s.merge_input_field()?;
    Ok(())
}

struct State<'a> {
    src: &'a Database,
    dst: &'a mut Database,
    user_mark_translate: HashMap<u32, u32>,
    location_translate: HashMap<u32, u32>,
    src_location_index: HashMap<u32, &'a Location>,
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
    key_symbol: Option<String>,
    meps_language: u32,
    r#type: u32,
}

impl From<&Location> for LocationValue {
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

impl State<'_> {
    fn merge_user_marks(&mut self) -> Result<()> {
        if self.src.user_marks.len() == 0 {
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

        for src in &self.src.user_marks {
            if let Some(&existing) = guid_map.get(&parse_guid(&src.user_mark_guid)?) {
                assert!(
                    self.user_mark_translate
                        .insert(src.user_mark_id, existing)
                        .is_none(),
                    "primary key user_mark violated"
                );
            } else {
                let src_id = src.user_mark_id;
                let mut clone = src.clone();
                clone.location_id = self.insert_location(src.location_id);
                user_mark_max_id += 1;
                clone.user_mark_id = user_mark_max_id;
                self.user_mark_translate.insert(src_id, clone.user_mark_id);
                self.dst.user_marks.push(clone);
            }
        }
        Ok(())
    }

    fn insert_location(&mut self, location_id: u32) -> u32 {
        if let Some(&translation) = self.location_translate.get(&location_id) {
            translation
        } else {
            let location = *self
                .src_location_index
                .get(&location_id)
                .expect("foreign key user_mark location violated");
            if let Some(&equivalent) = self.dst_location_value_index.get(&location.into()) {
                self.location_translate.insert(location_id, equivalent);
                equivalent
            } else {
                let mut clone = location.clone();
                self.location_max_id += 1;
                let new_id = self.location_max_id;
                clone.location_id = new_id;
                self.location_translate.insert(location_id, new_id);
                self.dst.locations.push(clone);
                new_id
            }
        }
    }

    fn merge_input_field(&mut self) -> Result<()> {
        if self.src.input_fields.len() > 0 {
            bail!("InputField merge not yet implemented");
        }
        // TODO: Merge InputField
        Ok(())
    }
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

    #[test]
    fn test_parse_guid() {
        let r = parse_guid("c88af989-da73-4745-bccc-8476f9950a3c").unwrap();
        dbg!(format!("{:x}", r));
        assert_eq!(r, 0xc88af989_da73_4745_bccc_8476f9950a3c);
    }
}
