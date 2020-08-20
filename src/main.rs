mod cleaner;
mod database;

use crate::cleaner::clean;
use crate::database::Database;
use anyhow::{anyhow, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use std::fs::File;
use std::io;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let input_files = std::env::args()
        .skip(1)
        .map(|p| File::open(p).context("Couldn't open input file"))
        .collect::<Result<Vec<File>>>()?;
    if input_files.len() < 2 {
        return Err(anyhow!("Provide at least 2 input files"));
    }

    for input in input_files {
        event!(Level::INFO, "Loading {:?}", &input);
        let mut backup = load(input)?;
        let name = &backup.manifest.name;
        event!(Level::INFO, "Backup name {}, cleaning", name);
        let rows_removed = clean(&mut backup.database)?;
        event!(
            Level::INFO,
            "Removed {} inaccessible rows from from backup {}",
            rows_removed,
            name
        );
    }

    event!(Level::INFO, "merge ");
    Ok(())
}

struct BackupFile {
    manifest: Manifest,
    database: Database,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    name: String,
    creation_date: String,
    version: i32,
    r#type: i32,
    user_data_backup: UserDataBackup,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct UserDataBackup {
    last_modified_date: String,
    device_name: String,
    hash: String,
    schema_version: i32,
}

const MANIFEST_ENTRY_NAME: &str = "manifest.json";
const DATABASE_ENTRY_NAME: &str = "userData.db";

fn load(file: impl io::Read + io::Seek) -> Result<BackupFile> {
    let mut zip = zip::ZipArchive::new(file).context("Unzip .jwlibrary")?;
    let manifest_entry = zip
        .by_name(MANIFEST_ENTRY_NAME)
        .context("Find manifest entry")?;
    let manifest: Manifest =
        serde_json::from_reader(manifest_entry).context("JSON decode manifest")?;
    let ver = manifest.version;
    ensure!(ver == 1, "Unsupported database version {}", ver);
    let ver = manifest.user_data_backup.schema_version;
    ensure!(ver == 8, "Unsupported database version {}", ver);

    let mut database_entry = zip
        .by_name(DATABASE_ENTRY_NAME)
        .context("Find database entry")?;
    let mut mem_file = Vec::with_capacity(database_entry.size() as _);
    io::copy(&mut database_entry, &mut mem_file).context("Read database to memory")?;

    let database = Database::read(mem_file)?;
    Ok(BackupFile { manifest, database })
}
