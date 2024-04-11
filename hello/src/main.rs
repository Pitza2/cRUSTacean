use serde::{Serialize, Deserialize};
use std::fs::OpenOptions;
use std::io::prelude::*;
#[derive(Serialize, Deserialize, Debug)]
struct FileData{
    name: String,
    files:Vec<String>
}
use std::{
    ffi::OsStr,
    fs::File,
    io::{Read, Seek}, iter::Zip, path::{self, PathBuf},
};

use zip::read::ZipFile;
use std::fs;    

/// Parametrul de tipul `impl Read + Seek` se numește "argument position impl trait" (APIT)
/// o formulare echivalentă ar fi `fn list_zip_contents<T: Read + Seek>(reader: T)`
/// `Read` și `Seek` sunt traits, care sunt oarecum similare cu interfețele din Java
///   o diferență este că traiturile nu sunt declarate direct de structuri (cum e în java `class C implements I`),
///   ci se pot declara separat: `impl Trait for Struct`
/// de asemenea generics în Rust diferă de cele din Java prin faptul că sunt monomorfice,
///   adică la compilare pentru o funcție generică se generează implementări separate pentru fiecare instanțiere cu argumente de tipuri diferite
///   (asta le aseamănă mai mult cu templates din C++)
/// https://doc.rust-lang.org/book/ch10-00-generics.html
///
/// deci practic lui `list_zip_contents` trebuie să-i dăm ca arugment o valoare al cărei tip implementează `Read` și `Seek`
///   un exemplu e `std::fs::File` (ar mai fi de exemplu `std::io::Cursor` cu care putem folosi un buffer din memorie)
/// 




fn list_zip_contents(reader: impl Read + Seek,pb:PathBuf) -> Result<FileData,Box<dyn std::error::Error>>{
    let mut zip = zip::ZipArchive::new(reader)?;

    let mut filenames:Vec<String>=Vec::new();

    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        filenames.push(file.name().to_string());
        //println!("\tFilename: {}", file.name());
    }
    let fd=FileData{
        name:pb.to_str().unwrap().to_string(),
        files:filenames
    };
    Ok(fd)
}

/// La `Box<dyn std::error::Error>` vedem o altă utilizare a traiturilor, de data asta sub formă de "trait objects".
/// Obiectele de tipul `dyn Trait` sunt un fel de pointeri polimorfici la structuri care implementează `Trait`.
/// Din nou putem face o paralelă la Java sau C++, unde o variabilă de tipul `Error e` poate să referențieze o
///   instanță a orcărei clase care implementează interfața (sau extinde clasa de bază) `Error`.
///
/// Valorile de typ `dyn Trait` trebuie mereu să fie în spatele unei referințe: `Box<dyn Trait>`, `&dyn Trait`, `&mut dyn Trait`, etc,
///  asta e pentru că nu știm exact ce obiect e în spatele pointerului și ce size are (se zice că trait objects sunt `unsized types`)
///
/// https://doc.rust-lang.org/book/ch17-02-trait-objects.html
///
/// `Box<dyn std::error::Error>` e util ca tip de eroare fiindcă în principiu toate erorile în Rust implementează `std::error::Error`
///   deci se pot converti implicit la `Box<dyn std::error::Error>` (ceea ce se întâmplă când folosim operatorul `?` de propagare).

fn getFileDataArray()->Result<Vec<FileData>,Box<dyn std::error::Error>>{
    let args: Vec<String> = std::env::args().collect();
    let mut fileDataArr:Vec<FileData>=Vec::new();
    let dir = &args[1];
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension() == Some(OsStr::new("zip")) {
            let file = File::open(&path)?;

           // println!("Contents of {:?}:", path);
            let fd=list_zip_contents(file,path)?;
            fileDataArr.push(fd);

        } else {
           // println!("Skipping {:?}", path);
        }
    }
    Ok(fileDataArr)
}

fn writeZipDataToFile(arr: Vec<FileData>,path:String)->Result<(), Box<dyn std::error::Error>>{
    let mut file = OpenOptions::new().create(true)
        .write(true)
        .open(path)
        .unwrap();

    
    for entry in arr{
        let serialized = serde_json::to_string(&entry).unwrap();
        if let Err(e) = writeln!(file,"{}\n", serialized) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }

    Ok(())
}

fn readZipDataFromJsonFile(path:String)->Result<(), Box<dyn std::error::Error>>{
    let mut fileRead = OpenOptions::new()
        .read(true)
        .open(path)
        .unwrap();
    let reader = std::io::BufReader::new(fileRead);
    let mut fileDataReadArr:Vec<FileData>=Vec::new();
    for line in reader.lines(){
        let deserialized: FileData=serde_json::from_str(line?.as_str()).unwrap();
        fileDataReadArr.push(deserialized);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
   let arr=getFileDataArray();
    
   writeZipDataToFile(arr?,"caca.txt".to_owned());

   let arr2=readZipDataFromJsonFile("caca.txt".to_owned());

   
    Ok(())
}