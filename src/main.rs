#[macro_use(bson, doc)]
extern crate bson;
extern crate chrono;
#[macro_use]
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate mongodb;
extern crate time;

use bson::Bson;
use bson::ordered::OrderedDocument;
use chrono::offset::utc::UTC;
use clap::{App, ArgMatches};
use mongodb::{Client, ClientOptions, ThreadedClient};
use mongodb::coll::options::{CursorType, FindOptions};
use mongodb::db::{Database, ThreadedDatabase};

mod analyzer;
mod demultiplexor;
mod error;
mod flag_manager;

use analyzer::{Analyzer, BayesianAnalyzer, ErrorAnalyzer};
use demultiplexor::Demultiplexor;
use error::TipupError;
use flag_manager::{Flag, FlagManager};

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Duration;

fn parse_args(matches: &ArgMatches) -> Result<(String, u16, String, String, String, String, String), TipupError> {
    let mongodb_ip_address = try!(value_t!(matches, "MONGODB_IP_ADDRESS", String));
    let mongodb_port = try!(value_t!(matches.value_of("MONGODB_PORT"), u16));
    let ca_file = try!(value_t!(matches.value_of("CA_FILE"), String));
    let certificate_file = try!(value_t!(matches.value_of("CERTIFICATE_FILE"), String));
    let key_file = try!(value_t!(matches.value_of("KEY_FILE"), String));
    let username = try!(value_t!(matches.value_of("USERNAME"), String));
    let password = try!(value_t!(matches.value_of("PASSWORD"), String));

    Ok((mongodb_ip_address, mongodb_port, ca_file, certificate_file, key_file, username, password))
}

fn main() {
    env_logger::init().unwrap();

    //parse arguments
    let yaml = load_yaml!("args.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let (mongodb_ip_address, mongodb_port, ca_file, certificate_file, key_file, username, password) = match parse_args(&matches) {
        Ok(args) => args,
        Err(e) => panic!("{}", e),
    };

    //connect to mongodb
    let client_options = ClientOptions::with_ssl(&ca_file, &certificate_file, &key_file, true);
    let client = match Client::connect_with_options(&mongodb_ip_address, mongodb_port, client_options)  {
        Ok(client) => client,
        Err(e) => panic!("{}", e),
    };

    //create flag manager and start
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut flag_manager = FlagManager::new();
        loop {
            let flag = match rx.recv() {
                Ok(flag) => flag,
                Err(e) => panic!("{}", e),
            };

            if let Err(e) = flag_manager.process_flag(&flag) {
                panic!("{}", e);
            }
        }
    });

    //create new demultiplexor
    let mut demultiplexor = Demultiplexor::new();
    let db = client.db("tipup");
    if let Err(e) = db.auth(&username, &password) {
        panic!("{}", e);
    }

    if let Err(e) = load_analyzers(&db, &mut demultiplexor, tx) {
        panic!("{}", e);
    }

    //demultiplexor loop
    let mut results_seen = HashMap::new();
    let db = client.db("proddle");
    if let Err(e) = db.auth(&username, &password) {
        panic!("{}", e);
    }

    loop {
        if let Err(e) = fetch_results(&db, &demultiplexor, &results_seen) {
            panic!("{}", e);
        }

        std::thread::sleep(Duration::new(60, 0))
    }
}

fn load_analyzers(db: &Database, demultiplexor: &mut Demultiplexor, tx: Sender<Flag>) -> Result<(), TipupError> {
    //query mongodb for analyzer definitions
    let cursor = try!(db.collection("analyzers").find(None, None));
    for document in cursor {
        //parse document
        let document = try!(document);
        let name = match document.get("name") {
            Some(&Bson::String(ref name)) => name,
            _ => return Err(TipupError::from("failed to parse analyzer name")),
        };

        let class = match document.get("class") {
            Some(&Bson::String(ref class)) => class,
            _ => return Err(TipupError::from("failed to parse analyzer class")),
        };

        let measurement = match document.get("measurement") {
            Some(&Bson::String(ref measurement)) => measurement,
            _ => return Err(TipupError::from("failed to parse analyzer measurement")),
        };

        let parameters = match document.get("parameters") {
            Some(&Bson::Array(ref parameters)) => parameters,
            _ => return Err(TipupError::from("failed to parse analyzer parameters")),
        };

        //create analyzer
        let analyzer = match class.as_ref() {
            "BayesianAnalyzer" => Box::new(try!(BayesianAnalyzer::new(parameters, tx.clone()))) as Box<Analyzer>,
            "ErrorAnalyzer" => Box::new(try!(ErrorAnalyzer::new(parameters, tx.clone()))) as Box<Analyzer>,
            _ => return Err(TipupError::from("unknown analyzer class")),
        };

        //add analyzer to demultiplexor
        try!(demultiplexor.add_analyzer(name.to_owned(), measurement.to_owned(), analyzer));
    }

    Ok(())
}

fn fetch_results(db: &Database, demultiplexor: &Demultiplexor, results_seen: &HashMap<String, i64>) -> Result<(), TipupError> {
    let start_time = UTC::now().timestamp() - (60 * 60 * 4);
    //TODO start_time is the lowest time in results_seen || 0
 
    let gte = doc! { "$gte" => start_time };
    let search_document = Some(doc! {
        "timestamp" => gte
    });

     //create find options
    let negative_one = -1;
    let sort_document = Some(doc! { "timestamp" => negative_one });
    let find_options = Some(FindOptions {
        allow_partial_results: false,
        no_cursor_timeout: false,
        op_log_replay: false,
        skip: 0,
        limit: 1,
        cursor_type: CursorType::NonTailable,
        batch_size: 0,
        comment: None,
        max_time_ms: None,
        modifiers: None,
        projection: None,
        sort: sort_document,
        read_preference: None,
    });

    let cursor = try!(db.collection("results").find(search_document, find_options));
    for document in cursor {
        let document = try!(document);
        println!("{:?}", document);

        //TODO compare timestamp and hostname to see if analysis is necessary & update results_seen
        //accordingly

        if let Err(e) = demultiplexor.send_result(&document) {
            panic!("{}", e);
        }
    }

    Ok(())
}
