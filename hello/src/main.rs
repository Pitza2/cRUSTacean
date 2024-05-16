#[macro_use] extern crate rocket;

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    time::Instant,
    sync::{Arc, RwLock},
};

use rocket::{fs::FileServer, serde::json::Json};

use serde::{Deserialize, Serialize};

type Term = String;
type DocumentId = String;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Default)]
struct IndexedData {
    terms_to_docs: HashMap<Term, Vec<DocumentId>>,
    idf: HashMap<Term, f64>,
    num_docs: usize,
}

impl IndexedData {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FileData {
    /// name of the zip archive
    name: DocumentId,
    /// list of files in the zip archive
    files: Vec<String>,
}

fn compute_idf(terms_to_docs: &HashMap<Term, Vec<DocumentId>>) -> HashMap<Term, f64> {
    let n = terms_to_docs.len() as f64;
    let mut terms_idf = HashMap::new();
    for (term, docs) in terms_to_docs {
        let nq = docs.len() as f64;
        let idf = ((n - nq + 0.5) / (nq + 0.5)).ln();
        terms_idf.insert(term.clone(), idf);
    }

    terms_idf
}

fn load_data(
    data_filename: &str,
    limit: Option<usize>,
) -> eyre::Result<IndexedData> {
    let file = File::open(data_filename)?;
    let reader = BufReader::new(file);

    let mut index = IndexedData::new();
    for line in reader.lines().take(limit.unwrap_or(usize::MAX)) {
        let line = line?;

        let fd: FileData = serde_json::from_str(&line)?;
        for file in fd.files {
            for term in file.split("/") {
                if let Some(set) = index.terms_to_docs.get_mut(term) {
                    if set.last() != Some(&fd.name) {
                        set.push(fd.name.clone());
                    }
                } else {
                    let mut set = Vec::new();
                    set.push(fd.name.clone());
                    index.terms_to_docs.insert(term.to_string(), set);
                }
            }
        }

        index.num_docs += 1;
    }

    index.idf = compute_idf(&index.terms_to_docs);
    Ok(index)
}

fn run_search(data: &IndexedData, terms: Vec<&str>) -> Vec<(DocumentId, f64)> {
    let mut counter: HashMap<DocumentId, u64> = HashMap::new();
    for term in &terms {
        if let Some(docs) = data.terms_to_docs.get(*term) {
            for doc in docs {
                let x = counter.entry(doc.to_string()).or_insert(0);
                *x += 1;
            }
        }
    }

    let mut scores: Vec<(DocumentId, f64)> = Vec::new();
    for (doc, cnt) in counter {
        scores.push((doc.to_string(), cnt as f64 / terms.len() as f64));
    }
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    scores
}


#[derive(Serialize)]
struct Greeting {
    message: String,
}

#[get("/")]
fn index() -> Json<Greeting> {
    Json(Greeting {
        message: "Hello, welcome to our server!".to_string(),
    })
}

#[derive(Default)]
struct ServerState {
    index: IndexedData,
}

#[rocket::main]
async fn main() -> eyre::Result<()> {

    let args: Vec<String> = std::env::args().collect();
    let data_filename = &args[1];
    let limit = args
        .get(2)
        .map(|x| usize::from_str_radix(&x, 10))
        .transpose()?;

    println!("loading {data_filename}...");
    let start = Instant::now();

    let data = load_data(data_filename, limit)?;

    let pair_count = data
        .terms_to_docs
        .iter()
        .map(|(_, docs)| docs.len())
        .sum::<usize>();
    println!(
        "loaded data for {} docs, {} terms, {} term-docid pairs, in {:.2}s",
        data.num_docs,
        data.terms_to_docs.len(),
        pair_count,
        start.elapsed().as_secs_f64(),
    );

    let start = Instant::now();
    let search = vec!["lombok", "AUTHORS", "README.md"];
    let matches = run_search(&data, search);
    println!(
        "search found {} matches in {:.2}s",
        matches.len(),
        start.elapsed().as_secs_f64(),
    );
    
    let server_state = Arc::new(RwLock::new(ServerState {
        index: data,
    }));
    rocket::build()
    .manage(server_state)
    .mount("/", routes![index])
    .mount("/dashboard", FileServer::from("static"))
    .ignite().await?
    .launch().await?;

    Ok(())
}
