#[macro_use(bson, doc)]
extern crate bson;
#[macro_use]
extern crate chan;
#[macro_use]
extern crate clap;
extern crate dbscan;
extern crate mongodb;
extern crate rustc_serialize;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;
extern crate slog_term;
extern crate time;

use bson::Bson;
use chan::Sender;
use clap::{App, ArgMatches};
use mongodb::{Client, ClientInner, ClientOptions, ThreadedClient};
use mongodb::coll::options::{CursorType, FindOneAndUpdateOptions, FindOptions};
use mongodb::db::{Database, ThreadedDatabase};
use slog::{DrainExt, Logger};

mod analyzer;
mod error;
mod event_manager;
mod flag_manager;
mod pipe;
mod result_window;

use analyzer::{Analyzer, ErrorAnalyzer, StdDevAnalyzer};
use error::TipupError;
use event_manager::EventManager;
use flag_manager::{Flag, FlagManager};
use pipe::Pipe;
use result_window::ResultWindow;

use std::sync::{Arc, RwLock};

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
    slog_scope::set_global_logger(Logger::root(slog_term::streamer().build().fuse(), o![]));

    //parse arguments
    let yaml = load_yaml!("args.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let (mongodb_ip_address, mongodb_port, ca_file, certificate_file, key_file, username, password) = match parse_args(&matches) {
        Ok(args) => args,
        Err(e) => panic!("{}", e),
    };

    //connect to mongodb
    let client = match initialize_mongodb_client(&mongodb_ip_address, mongodb_port, &ca_file, &certificate_file, &key_file) {
        Ok(client) => client,
        Err(e) => panic!("{}", e),
    };
    
    let (tipup_db, proddle_db) = (client.db("tipup"), client.db("proddle"));
    if let Err(e) = tipup_db.auth(&username, &password) {
        panic!("{}", e);
    }

    if let Err(e) = proddle_db.auth(&username, &password) {
        panic!("{}", e);
    }

    //create pipe and result_window
    let result_window = Arc::new(RwLock::new(ResultWindow::new()));
    let (flag_tx, flag_rx) = chan::sync(50);
    let mut pipe = Pipe::new();
    if let Err(e) = load_analyzers(&proddle_db, &tipup_db, &mut pipe, flag_tx, result_window.clone()) {
        panic!("{}", e);
    }

    //populate result window
    info!("initializing result window");
    {
        let mut result_window = result_window.write().unwrap();
        if let Err(e) = result_window.initialize(&proddle_db) {
           panic!("{}", e);
        }
    }

    //create flag manager and start
    info!("initializing flag manager");
    std::thread::spawn(move || {
        let client = match initialize_mongodb_client(&mongodb_ip_address, mongodb_port, &ca_file, &certificate_file, &key_file) {
            Ok(client) => client,
            Err(e) => panic!("{}", e),
        };

        let tipup_db = client.db("tipup");
        if let Err(e) = tipup_db.auth(&username, &password) {
            panic!("{}", e);
        }

        let mut flag_manager = FlagManager::new(&tipup_db);
        loop {
            chan_select! {
                flag_rx.recv() -> flag => {
                    match flag {
                        Some(flag) => {
                            if let Err(e) = flag_manager.process_flag(&flag) {
                                panic!("{}", e);
                            }
                        },
                        None => continue,
                    }
                }
            }
        }
    });

    //create event manager
    info!("initializing event manager");
    let event_manager = EventManager::new(604800); //7 days = 604800 seconds

    //start command loop
    info!("started");
    let fetch_results_tick = chan::tick_ms(5 * 60 * 1000); //execute every 5 minutes
    //let analyze_event_tick = chan::tick_ms(10 * 60 * 1000); //execute every 10 minutes
    let analyze_event_tick = chan::tick_ms(1000); //execute every 10 minutes
    loop {
        chan_select! {
            fetch_results_tick.recv() => {
                if let Err(e) = fetch_results(&proddle_db, &tipup_db, &pipe, result_window.clone()) {
                    error!("{}", e);
                }
            },
            analyze_event_tick.recv() => {
                if let Err(e) = event_manager.execute(&tipup_db) {
                    error!("{}", e);
                }
            },
        }
    }
}

fn initialize_mongodb_client(mongodb_ip_address: &str, mongodb_port: u16, ca_file: &str, certificate_file: &str, key_file: &str) -> Result<Arc<ClientInner>, mongodb::Error> {
    if ca_file.eq("") && certificate_file.eq("") && key_file.eq("") {
        Client::connect(mongodb_ip_address, mongodb_port)
    } else {
        let client_options = ClientOptions::with_ssl(ca_file, certificate_file, key_file, true);
        Client::connect_with_options(mongodb_ip_address, mongodb_port, client_options)
    }
}

fn load_analyzers(_: &Database, tipup_db: &Database, pipe: &mut Pipe, flag_tx: Sender<Flag>, result_window: Arc<RwLock<ResultWindow>>) -> Result<(), TipupError> {
    //query mongodb for analyzer definitions
    let mut count = 0;
    let cursor = try!(tipup_db.collection("analyzers").find(None, None));
    for document in cursor {
        //parse document
        let document = try!(document);
        info!("loading analyzer: {:?}", document);

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
            Some(&Bson::Document(ref parameters)) => parameters,
            _ => return Err(TipupError::from("failed to parse analyzer parameters")),
        };

        //create analyzer
        let analyzer = match class.as_ref() {
            "ErrorAnalyzer" => Box::new(try!(ErrorAnalyzer::new(name, flag_tx.clone()))) as Box<Analyzer>,
            "StdDevAnalyzer" => Box::new(try!(StdDevAnalyzer::new(name, parameters, result_window.clone(), flag_tx.clone()))) as Box<Analyzer>,
            _ => return Err(TipupError::from("unknown analyzer class")),
        };

        //add analyzer to pipe
        try!(pipe.add_analyzer(name.to_owned(), measurement.to_owned(), analyzer));
        count += 1;
    }

    if count > 0 {
        info!("loaded {} analyzer(s)", count);
    }
    Ok(())
}

fn fetch_results(proddle_db: &Database, tipup_db: &Database, pipe: &Pipe, result_window: Arc<RwLock<ResultWindow>>) -> Result<(), TipupError> {
    //iterate over distinct hostnames for results
    let mut count = 0;
    let hostname_cursor = try!(proddle_db.collection("results").distinct("hostname", None, None));
    for hostname_document in hostname_cursor {
        let hostname = match hostname_document {
            Bson::String(ref hostname) => hostname,
            _ => continue,
        };

        //query tipup db for timestamp of last seen result
        let tipup_search_document = Some(doc!("hostname" => hostname));
        let document = try!(tipup_db.collection("last_seen_result").find_one(tipup_search_document, None));
        let timestamp = match document {
            Some(document) => {
                match document.get("timestamp") {
                    Some(&Bson::I64(timestamp)) => timestamp,
                    _ => return Err(TipupError::from(format!("failed to parse 'timestamp' value in tipup.last_seen_result for host '{}'", hostname))),
                }
            },
            None => 0,
        };

        //iterate over newest results
        let gt = doc!("$gt" => timestamp);
        let proddle_search_document = Some(doc!(
            "hostname" => hostname,
            "timestamp" => gt
        ));

        //create find options
        let negative_one = -1;
        let sort_document = Some(doc!("timestamp" => negative_one));
        let find_options = Some(FindOptions {
            allow_partial_results: false,
            no_cursor_timeout: false,
            oplog_replay: false,
            skip: None,
            limit: None,
            cursor_type: CursorType::NonTailable,
            batch_size: None,
            comment: None,
            max_time_ms: None,
            modifiers: None,
            projection: None,
            sort: sort_document,
            read_preference: None,
        });

        //iterate over new results
        let cursor = try!(proddle_db.collection("results").find(proddle_search_document, find_options));
        let mut max_timestamp = -1;
        for document in cursor {
            let document = try!(document);
            if let Err(e) = pipe.send_result(&document) {
                panic!("document:{:?} err:{}", document, e);
            }

            match document.get("timestamp") {
                Some(&Bson::I64(result_timestamp)) => max_timestamp = std::cmp::max(max_timestamp, result_timestamp),
                _ => return Err(TipupError::from("failed to parse 'timestamp' value in result")),
            }

            //add result to result window
            {
                let mut result_window = result_window.write().unwrap();
                try!(result_window.add_result(document))
            }

            count += 1;
        }

        //update tipup db with most recenlty seen result timestamp
        if max_timestamp != -1 {
            let search_document = doc!("hostname" => hostname);
            let update_timestamp_document = doc!("timestamp" => max_timestamp);
            let update_document = doc!("$set" => update_timestamp_document);
            let update_options = Some(FindOneAndUpdateOptions {
                return_document: None,
                max_time_ms: None,
                projection: None,
                sort: None,
                upsert: Some(true),
                write_concern: None,
            });

            try!(tipup_db.collection("last_seen_result").find_one_and_update(search_document, update_document, update_options));
        }
    }

    if count > 0 {
        info!("fetched {} new result(s)", count);
    }
    Ok(())
}
