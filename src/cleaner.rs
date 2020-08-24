use crate::Database;
use std::collections::HashSet;
use tracing::{event, Level};

pub fn clean(database: &mut Database) -> usize {
    let mut c = Cleaner {
        database,
        rows_removed: 0,
    };
    c.clean_block_ranges();
    c.clean_locations();
    c.rows_removed
}

struct Cleaner<'a> {
    database: &'a mut Database,
    rows_removed: usize,
}

impl Cleaner<'_> {
    fn clean_block_ranges(&mut self) {
        let ranges = &mut self.database.block_ranges;
        if !ranges.is_empty() {
            event!(Level::DEBUG, "Process {} block ranges", ranges.len());
            let user_mark_ids: HashSet<u32> = self
                .database
                .user_marks
                .iter()
                .map(|u| u.user_mark_id)
                .collect();
            let mut retained = Vec::with_capacity(ranges.len());
            let mut user_mark_ids_found = HashSet::with_capacity(ranges.len());
            for range in ranges.iter().rev() {
                if !user_mark_ids.contains(&range.user_mark_id) {
                    event!(
                        Level::DEBUG,
                        "Removing redundant range: {}",
                        range.block_range_id
                    );
                } else if user_mark_ids_found.insert(range.user_mark_id) {
                    retained.push(range.clone());
                } else {
                    event!(
                        Level::DEBUG,
                        "Removing redundant range (duplicate UserMarkId): {}",
                        range.block_range_id
                    );
                    // TODO: Why should these ranges be discarded?
                }
            }
            self.rows_removed += ranges.len() - retained.len();
            *ranges = retained;
        }
    }

    fn clean_locations(&mut self) {
        let in_use = self.location_ids_in_use();
        let locations = &mut self.database.locations;
        let len_before = locations.len();
        // TODO: Optimize, use drain_filter once stable
        *locations = locations
            .iter()
            .rev()
            .filter(|l| {
                let c = in_use.contains(&l.location_id);
                if !c {
                    event!(
                        Level::DEBUG,
                        "Removing redundant location ID {}",
                        l.location_id
                    );
                }
                c
            })
            .cloned()
            .collect();
        self.rows_removed += len_before - locations.len();
    }

    fn location_ids_in_use(&self) -> HashSet<u32> {
        let mut result = HashSet::with_capacity(self.database.locations.len());
        self.database.bookmarks.iter().for_each(|b| {
            result.insert(b.location_id);
            result.insert(b.publication_location_id);
        });
        self.database
            .notes
            .iter()
            .filter_map(|n| n.location_id)
            .for_each(|id| {
                result.insert(id);
            });
        self.database.user_marks.iter().for_each(|u| {
            result.insert(u.location_id);
        });
        self.database
            .tag_maps
            .iter()
            .filter_map(|t| t.location_id)
            .for_each(|id| {
                result.insert(id);
            });
        event!(Level::DEBUG, "Found {} location IDs in use", result.len());
        result
    }
}
