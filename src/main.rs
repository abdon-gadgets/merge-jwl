mod cleaner;
mod database;
mod merger;

use crate::cleaner::clean;
use crate::database::Database;
use crate::merger::merge_databases;
use anyhow::{anyhow, bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use std::io::{self, Write};

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    use std::fs::{File, OpenOptions};
    tracing_subscriber::fmt::init();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 2 {
        return Err(anyhow!("Provide at least 2 input files"));
    }
    let input_files = args
        .iter()
        .map(|p| File::open(p).context("Couldn't open input file"))
        .collect::<Result<Vec<File>>>()?;

    let (manifest, mem_file) = run(input_files)?;
    let mut path = std::path::PathBuf::from(".");
    path.set_file_name(&manifest.name);
    path.set_extension("jwlibrary");
    let mut output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .context("Couldn't open output file")?;
    compress(manifest, mem_file, &mut output_file)?;

    event!(Level::INFO, "Finished");
    Ok(())
}

fn run(input_files: Vec<impl io::Read + io::Seek + std::fmt::Debug>) -> Result<BackupVec> {
    let mut databases = Vec::with_capacity(input_files.len());
    let mut manifests = Vec::with_capacity(input_files.len());
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
        databases.push(backup.database);
        manifests.push(backup.manifest);
    }

    let database = merge_databases(databases.into_iter())?;
    let mem_file = database.serialize()?;
    let manifest = update_manifest(&manifests, compute_hash(&mem_file));
    Ok((manifest, mem_file))
}

type BackupVec = (Manifest, Vec<u8>);

struct Backup {
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
    database_name: String,
    hash: String,
    schema_version: i32,
}

const MANIFEST_ENTRY_NAME: &str = "manifest.json";

fn load(file: impl io::Read + io::Seek) -> Result<Backup> {
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
        .by_name(&manifest.user_data_backup.database_name)
        .context("Find database entry")?;
    let mut mem_file = Vec::with_capacity(database_entry.size() as _);
    io::copy(&mut database_entry, &mut mem_file).context("Read database to memory")?;
    ensure!(
        compute_hash(&mem_file) == manifest.user_data_backup.hash,
        "Hash mismatch"
    );

    let database = Database::read(mem_file)?;
    Ok(Backup { manifest, database })
}

fn compute_hash(file: &[u8]) -> String {
    use sha2::Digest;
    let hash = sha2::Sha256::digest(file);
    format!("{:x}", hash)
}

fn update_manifest(originals: &[Manifest], hash: String) -> Manifest {
    let date = now();
    // pick the first manifest as the basis for the manifest in the final merged file
    let base = &originals[0];
    let mut device_name: Vec<&str> = originals
        .iter()
        .map(|d| d.user_data_backup.device_name.as_str())
        .collect();
    device_name.dedup();
    let device_name = format!("{} (merge-jwl)", device_name.join("🔁"));
    Manifest {
        name: format!("UserDataBackup_{}_Merge", &date),
        creation_date: date,
        user_data_backup: UserDataBackup {
            last_modified_date: base.user_data_backup.last_modified_date.to_string(),
            device_name,
            hash,
            database_name: base.user_data_backup.database_name.to_string(),
            schema_version: base.user_data_backup.schema_version,
        },
        version: base.version,
        r#type: base.r#type,
    }
}

fn compress(
    manifest: Manifest,
    database: Vec<u8>,
    file: &mut (impl io::Write + io::Seek),
) -> Result<()> {
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        // TODO .last_modified_time()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.set_comment("");
    zip.start_file(MANIFEST_ENTRY_NAME, options)?;
    serde_json::to_writer(&mut zip, &manifest)?;

    zip.start_file(&manifest.user_data_backup.database_name, options)?;
    zip.write_all(&database)?;
    zip.finish()?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn now() -> String {
    chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now())
        .format("%F")
        .to_string()
}

#[cfg(test)]
mod test {
    use crate::compute_hash;

    #[test]
    fn test_hash() {
        let hash = compute_hash(b"hello world");
        assert_eq!(
            &hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
