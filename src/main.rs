mod cleaner;
mod database;
mod merger;

use crate::cleaner::clean;
use crate::database::Database;
use crate::merger::merge_databases;
use anyhow::{anyhow, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use zip::DateTime;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        return Err(anyhow!("Provide at least 2 input files and 1 output file"));
    }
    let input_files = args[..args.len() - 1]
        .iter()
        .map(|p| File::open(p).context("Couldn't open input file"))
        .collect::<Result<Vec<File>>>()?;
    let mut output_file =
        File::create(args.last().unwrap()).context("Couldn't open output file")?;

    let mut originals = Vec::with_capacity(input_files.len());
    for input in input_files {
        event!(Level::INFO, "Loading {:?}", &input);
        let mut backup = load(input)?;
        let name = &backup.manifest.name;
        event!(Level::INFO, "Backup name {}, cleaning", name);
        let rows_removed = clean(&mut backup.database);
        event!(
            Level::INFO,
            "Removed {} inaccessible rows from from backup {}",
            rows_removed,
            name
        );
        originals.push(backup);
    }
    // pick the first manifest as the basis for the manifest in the final merged file
    let merged = BackupFile {
        manifest: originals[0].manifest.clone(),
        database: merge_databases(originals.into_iter().map(|o| o.database))?,
    };
    compress(merged, &mut output_file);

    event!(Level::INFO, "Merge");
    Ok(())
}

struct BackupFile {
    manifest: Manifest,
    database: Database,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    name: String,
    creation_date: String,
    version: i32,
    r#type: i32,
    user_data_backup: UserDataBackup,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

fn compress(backup: BackupFile, file: &mut (impl io::Write + io::Seek)) -> Result<()> {
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        // TODO .last_modified_time()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.set_comment("");
    zip.start_file(MANIFEST_ENTRY_NAME, options)?;
    serde_json::to_writer(&mut zip, &backup.manifest);

    let mem_file = backup.database.serialize()?;
    zip.start_file(DATABASE_ENTRY_NAME, options)?;
    zip.write_all(&mem_file)?;
    zip.finish()?;
    Ok(())
}
