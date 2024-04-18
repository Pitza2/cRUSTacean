use std::{
    collections::HashSet, fs::File, io::{BufRead, BufReader}, time::Instant
};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct FileData {
    /// name of the zip archive
    name: String,
    /// list of files in the zip archive
    files: Vec<String>,
}
type Term=String;
type DocumentId=String;
type IndexType=HashMap<Term,HashSet<DocumentId>>;

fn load_data(data_filename: &str) -> Result<IndexType, Box<dyn std::error::Error>> {
    let file = File::open(data_filename)?;
    let reader = BufReader::new(file);


    let mut data : IndexType= IndexType::new();
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        let fdata:FileData=serde_json::from_str(line)?;
        
        for file in fdata.files{
            for term in file.split("/"){
                let mut set:HashSet<String>=HashSet::new();
                set.insert(fdata.name.to_string());
                data.entry(term.to_string()).and_modify(|set| {set.insert(fdata.name.to_string());})
                .or_insert(set);
            }
        }
    }
    Ok(data)
}
fn run_search(data: &IndexType,terms:Vec<&str>)->Result<HashMap<DocumentId,u64>, Box<dyn std::error::Error>>{
    let mut counter : HashMap<DocumentId,u64>=HashMap::new();
    for term in terms{
        let x= data.get(term).unwrap();
        for val in x {
            counter.entry(val.to_string()).and_modify(|num| *num+=1).or_insert(1);
        }
    }
    Ok(counter)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let data_filename = &args[1];
    let data = load_data(&data_filename)?;
   
    println!("loaded data for {} files", data.len());
    // let mut nr=0;
    // for kv in data{
    //     println!("{}: {:?}\n\n\n",kv.0,kv.1);
    //     nr+=1;
    //     if nr == 10{break;}
    // }
    let start =Instant::now();

    let rez=run_search(&data, vec!["lombok","AUTHORS","README.md"]).unwrap();
    for kv in rez{
        println!("in {} found {}/3 items",kv.0,kv.1);
    }
    println!("elapsed time {:?}",start.elapsed());

    Ok(())
}