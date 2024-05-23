#[macro_use] extern crate rocket;

use std::{
    borrow::Borrow, collections::HashMap, fs::File, io::{BufRead, BufReader}, sync::{Arc, RwLock}, time::Instant, vec
};
use rocket_okapi::{openapi, openapi_get_routes, swagger_ui::*, JsonSchema};

use rocket::{fs::FileServer, http::hyper::server::{self, Server}, serde::json::Json,State};

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

fn run_search(data: &IndexedData, terms: Vec<&str>) -> Vec<SearchMatch> {
    let mut counter: HashMap<DocumentId, u64> = HashMap::new();
    for term in &terms {
        if let Some(docs) = data.terms_to_docs.get(*term) {
            for doc in docs {
                let x = counter.entry(doc.to_string()).or_insert(0);
                *x += 1;
            }
        }
    }

    let mut scores: Vec<SearchMatch> = Vec::new();
    for (doc, cnt) in counter {//(doc.to_string(), cnt as f64 / terms.len() as f64)
        scores.push(SearchMatch{md5:doc.to_string(),score:cnt as f64/terms.len() as f64});
    }
    scores.sort_by(|a, b| b.score.total_cmp(&a.score));
    scores
}


#[derive(Serialize)]
struct Greeting {
    message: String,
}
#[derive(Deserialize, JsonSchema)]
struct SearchData{
    terms:Vec<String>,
}
#[derive(Serialize,JsonSchema)]
struct SearchResult{
    matches:Vec<SearchMatch>,
    total:u64
}
#[derive(Serialize,JsonSchema)]
struct SearchMatch{
    md5:DocumentId,
    score:f64
}
#[openapi(tag = "Users")]
#[post("/search",data="<req>")]
fn search(req:Json<SearchData>,server_state: &State<Arc<RwLock<ServerState>>>) -> Result<Json<SearchResult>,String>{
    let terms =req.terms.clone();
    let vec_of_strs: Vec<&str> = terms.iter().map(|s| s.as_str()).collect();
    let result=run_search(&server_state.read().unwrap().index, vec_of_strs);
    let l=result.len();
    let sr=SearchResult{matches:result,total:l as u64};
    Ok(Json(sr))
}
#[openapi(skip)]
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
    //.mount("/", routes![index, search])
    .mount("/dashboard", FileServer::from("static"))
    .mount(
        "/",
        openapi_get_routes![
            index,
            search
        ],
    )
    .mount(
        "/swagger-ui/",
        make_swagger_ui(&SwaggerUIConfig {
            url: "../openapi.json".to_owned(),
            ..Default::default()
        }),
    )
    .ignite().await?
    .launch().await?;

    Ok(())
}
