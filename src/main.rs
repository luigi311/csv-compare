use clap::Parser;
use std::collections::HashSet;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufWriter, Error, Write};
use std::path::PathBuf;
use csv::Reader;
use itertools::Itertools;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

struct FileHashes {
    file: PathBuf,
    has_headers: bool,
    hashes: Vec<u64>,
    hash_set: HashSet<u64>, // HashSet has better lookup performance compared to vec contains
}

struct CompareResults {
    from: PathBuf,
    to: PathBuf,
    missing: Vec<usize>,
    exists: Vec<usize>
}

fn process_file(file: &PathBuf) -> Result<FileHashes, Error> {
    println!("Processing {}", file.display());
    let mut hashes: Vec<u64> = Vec::new();

    let mut reader = Reader::from_path(file).expect("Unable to open file");


    for (idx, row) in reader.records().enumerate() {
        let mut hasher = DefaultHasher::new();

        let record = row?;
        for column in record.iter() {
            column.hash(&mut hasher);
        }
        hashes.push(hasher.finish());

        if (idx % 100_000 == 0) && (idx != 0) {
            println!("Progress line {}", idx);
        }
    }

    let hash_set: HashSet<u64> = hashes.iter().copied().collect();
    Ok(FileHashes{file: file.clone(), has_headers: reader.has_headers(), hashes: hashes, hash_set: hash_set})
}


fn generate_file_hashes(files: &[PathBuf]) -> Result<Vec<FileHashes>, Error> {
    let mut file_hashes: Vec<FileHashes> = Vec::new();

    for file in files {
        file_hashes.push(process_file(file)?);
    }

    Ok(file_hashes)
}

fn compare_exists_missing(file1: &FileHashes, file2: &FileHashes) -> CompareResults {
    println!("Comparing {} to {}", file1.file.display(), file2.file.display());

    let mut exists: Vec<usize> = Vec::new();
    let mut missing: Vec<usize> = Vec::new();
    let increment: usize = if file1.has_headers { 2 } else { 1 };

    for (index, hash) in file1.hashes.iter().enumerate() {
        if file2.hash_set.contains(hash) {
            exists.push(index + increment)
        } else {
            missing.push(index + increment)
        }
    }

    CompareResults { from: file1.file.clone(), to: file2.file.clone(), missing: missing, exists }
}

fn write_file(file_name: String, list: Vec<usize>) -> Result<(), Error> {
    let file = File::create(file_name)?;
    let mut writer = BufWriter::new(file);
    for row in list {
        writeln!(writer, "{}", row)?;
    }
    writer.flush()?;

    Ok(())
}

fn main() {
    let args = Cli::parse();
    let files = &args.files;

    let file_hashes: Vec<FileHashes> = generate_file_hashes(files).unwrap();
    let pairs = file_hashes.iter().tuple_combinations::<(&FileHashes, &FileHashes)>();
    let mut results: Vec<CompareResults> = Vec::new();

    for (file1, file2) in pairs {
        results.push(compare_exists_missing(file1, file2));
        results.push(compare_exists_missing(file2, file1));
    }

    for result in results {
        let file1 = result.from.file_stem().unwrap();
        let file2 = result.to.file_stem().unwrap();
        let missing_file_name = format!("missing_{}_in_{}", file1.to_string_lossy(), file2.to_string_lossy());
        let exists_file_name = format!("exists_{}_in_{}", file1.to_string_lossy(), file2.to_string_lossy());

        write_file(missing_file_name, result.missing).unwrap();
        write_file(exists_file_name, result.exists).unwrap();
    }
}
