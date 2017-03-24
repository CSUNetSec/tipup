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

fn parse_args(matches: &ArgMatches) -> Result<(String, u16, String, String, String, String, String, u32, u32), TipupError> {
    let mongodb_ip_address = try!(value_t!(matches, "MONGODB_IP_ADDRESS", String));
    let mongodb_port = try!(value_t!(matches.value_of("MONGODB_PORT"), u16));
    let ca_file = try!(value_t!(matches.value_of("CA_FILE"), String));
    let certificate_file = try!(value_t!(matches.value_of("CERTIFICATE_FILE"), String));
    let key_file = try!(value_t!(matches.value_of("KEY_FILE"), String));
    let username = try!(value_t!(matches.value_of("USERNAME"), String));
    let password = try!(value_t!(matches.value_of("PASSWORD"), String));
    let update_flags_interval = try!(value_t!(matches.value_of("UPDATE_FLAGS_INTERVAL"), u32));
    let update_events_interval = try!(value_t!(matches.value_of("UPDATE_EVENTS_INTERVAL"), u32));

    Ok((mongodb_ip_address, mongodb_port, ca_file, certificate_file, key_file, username, password, update_flags_interval, update_events_interval))
}

fn main() {
    slog_scope::set_global_logger(Logger::root(slog_term::streamer().build().fuse(), o![]));

    //parse arguments
    let yaml = load_yaml!("args.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let (mongodb_ip_address, mongodb_port, ca_file, certificate_file, key_file, username, password, update_flags_interval, update_events_interval) = match parse_args(&matches) {
        Ok(args) => args,
        Err(e) => panic!("{}", e),
    };

    //connect to mongodb
    let client = match initialize_mongodb_client(&mongodb_ip_address, mongodb_port, &ca_file, &certificate_file, &key_file) {
        Ok(client) => client,
        Err(e) => panic!("{}", e),
    };
    
    //create pipe and result_window
    let result_window = Arc::new(RwLock::new(ResultWindow::new()));
    let (flag_tx, flag_rx) = chan::sync(50);
    let mut pipe = Pipe::new();
    {
        let db = match initialize_db(&client, "proddle", &username, &password) {
            Ok(db) => db,
            Err(e) => panic!("{}", e),
        };

        if let Err(e) = load_analyzers(&db, &mut pipe, flag_tx, result_window.clone()) {
            panic!("{}", e);
        }

        info!("initializing result window");
        let mut result_window = result_window.write().unwrap();
        if let Err(e) = result_window.initialize(&db) {
           panic!("{}", e);
        }
    }

    //create flag manager and start
    info!("initializing flag manager");
    let (thread_username, thread_password) = (username.clone(), password.clone());
    std::thread::spawn(move || {
        let mut flag_buffer = Vec::new();
        let mut flag_manager = FlagManager::new();
        let process_flag_tick = chan::tick_ms(5 * 1000);

        let client = match initialize_mongodb_client(&mongodb_ip_address, mongodb_port, &ca_file, &certificate_file, &key_file) {
            Ok(client) => client,
            Err(e) => panic!("{}", e),
        };

        loop {
            chan_select! {
                flag_rx.recv() -> flag => {
                    if let Some(flag) = flag {
                        flag_buffer.push(flag);
                    }
                },
                process_flag_tick.recv() => {
                    if flag_buffer.len() > 0 {
                        let db = match initialize_db(&client, "proddle", &thread_username, &thread_password) {
                            Ok(db) => db,
                            Err(e) => {
                                error!("{}", e);
                                continue;
                            },
                        };

                        for flag in flag_buffer.iter() {
                            if let Err(e) = flag_manager.process_flag(flag, &db) {
                                error!("{}", e);
                            }
                        }
                        
                        info!("wrote {} new flag(s)", flag_buffer.len());
                        flag_buffer.clear();
                    }
                },
            }
        }
    });

    //create event manager
    info!("initializing event manager");
    let event_manager = EventManager::new(604800); //7 days = 604800 seconds

    //start command loop
    info!("TIPUP STARTED");
    let update_flags_tick = chan::tick_ms(update_flags_interval * 1000);
    let update_events_tick = chan::tick_ms(update_events_interval * 1000);
    loop {
        chan_select! {
            update_flags_tick.recv() => {
                let db = match initialize_db(&client, "proddle", &username, &password) {
                    Ok(db) => db,
                    Err(e) => {
                        error!("{}", e);
                        continue;
                    },
                };

                if let Err(e) = fetch_results(&db, &pipe, result_window.clone()) {
                    error!("{}", e);
                }
            },
            update_events_tick.recv() => {
                /*let db = match initialize_db(&client, "proddle", &username, &password) {
                    Ok(db) => db,
                    Err(e) => {
                        error!("{}", e);
                        continue;
                    },
                };

                if let Err(e) = event_manager.execute(&db) {
                    error!("{}", e);
                }*/
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

fn initialize_db(client: &Client, db_name: &str, username: &str, password: &str) -> Result<Database, TipupError> {
    let db = client.db(db_name);
    try!(db.auth(&username, &password));
    Ok(db)
}

fn load_analyzers(db: &Database, pipe: &mut Pipe, flag_tx: Sender<Flag>, result_window: Arc<RwLock<ResultWindow>>) -> Result<(), TipupError> {
    //query mongodb for analyzer definitions
    let mut count = 0;
    let cursor = try!(db.collection("analyzers").find(None, None));
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

        let status = match document.get("status") {
            Some(&Bson::String(ref status)) => status,
            _ => return Err(TipupError::from("failed to parse analyzer status")),
        };

        let measurement_class = match document.get("measurement_class") {
            Some(&Bson::String(ref measurement_class)) => measurement_class,
            _ => return Err(TipupError::from("failed to parse analyzer measurement_class")),
        };

        let fields: Vec<String> = match document.get("fields") {
            Some(&Bson::Array(ref fields)) => fields.iter().map(|x| x.to_string().replace("\"", "")).collect(),
            _ => return Err(TipupError::from("failed to parse analyzer fields")),
        };

        let parameters = match document.get("parameters") {
            Some(&Bson::Array(ref parameters)) => parameters,
            _ => return Err(TipupError::from("failed to parse analyzer parameters")),
        };

        //create analyzer
        let analyzer = match class.as_ref() {
            "ErrorAnalyzer" => Box::new(try!(ErrorAnalyzer::new(name, status, fields, flag_tx.clone()))) as Box<Analyzer>,
            //"StdDevAnalyzer" => Box::new(try!(StdDevAnalyzer::new(name, parameters, result_window.clone(), flag_tx.clone()))) as Box<Analyzer>,
            _ => return Err(TipupError::from("unknown analyzer class")),
        };

        //add analyzer to pipe
        try!(pipe.add_analyzer(name.to_owned(), measurement_class.to_owned(), analyzer));
        count += 1;
    }

    if count > 0 {
        info!("loaded {} analyzer(s)", count);
    }

    Ok(())
}

fn fetch_results(db: &Database, pipe: &Pipe, result_window: Arc<RwLock<ResultWindow>>) -> Result<(), TipupError> {
    //iterate over distinct hostnames for measurements
    let mut count = 0;
    let hostname_cursor = try!(db.collection("measurements").distinct("vantage_hostname", None, None));
    for hostname_document in hostname_cursor {
        let hostname = match hostname_document {
            Bson::String(ref hostname) => hostname,
            _ => continue,
        };

        //query db for timestamp of last seen result
        let search_document = Some(doc!("vantage_hostname" => hostname));
        let document = try!(db.collection("analyzed_measurements").find_one(search_document, None));
        let timestamp = match document {
            Some(document) => {
                match document.get("timestamp") {
                    Some(&Bson::I64(timestamp)) => timestamp,
                    _ => return Err(TipupError::from(format!("failed to parse 'timestamp' value in analyzed_measurements for host '{}'", hostname))),
                }
            },
            None => 0,
        };

        //iterate over newest measurements
        let gt = doc!("$gt" => timestamp);
        let search_document = Some(doc!(
            "vantage_hostname" => hostname,
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

        //iterate over new measurements
        let cursor = try!(db.collection("measurements").find(search_document, find_options));
        let mut max_timestamp = -1;
        for document in cursor {
            let document = try!(document);
            if let Err(e) = pipe.send_measurement(&document) {
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

        //update db with most recenlty analyzed result timestamp
        if max_timestamp != -1 {
            let search_document = doc!("vantage_hostname" => hostname);
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

            try!(db.collection("analyzed_measurements").find_one_and_update(search_document, update_document, update_options));
        }
    }

    if count > 0 {
        info!("fetched {} new measurements(s)", count);
    }

    Ok(())
}
