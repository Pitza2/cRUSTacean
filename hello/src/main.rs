use std::env;
use std::fs::File;

use std::io::prelude::*;

use zip::ZipArchive;
fn list_zip_contents(reader: impl Read + Seek) -> zip::result::ZipResult<()> {
    let mut zip = zip::ZipArchive::new(reader)?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        print!("Filename: {} ", file.name());
        print!("size: {} ", file.size());
        println!("compressionMethod?: {}", file.compression());
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    if !args[1].ends_with(".zip") {
        return;
    }
    let file = File::open(&args[1]).unwrap();
    list_zip_contents(file);
}