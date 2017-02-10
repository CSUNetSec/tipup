#[macro_use(bson, doc)]
extern crate bson;
#[macro_use]
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate mongodb;
extern crate time;

use bson::Bson;
use bson::ordered::OrderedDocument;
use clap::{App, ArgMatches};
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase;

mod analyzer;
mod demultiplexor;
mod error;

use error::TipupError;
use demultiplexor::Demultiplexor;

use std::sync::{Arc, Mutex};
use std::time::Duration;

fn parse_args(matches: &ArgMatches) -> Result<(), TipupError> {
    Ok(())
}

fn main() {
    env_logger::init().unwrap();

    //parse arguments
    let yaml = load_yaml!("main_args.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let () = match parse_args(&matches) {
        Ok(tuple) => tuple,
        Err(e) => panic!("{}", e),
    };

    //create new demultiplexor
    let mut demultiplexor = Arc::new(Mutex::new(Demultiplexor::new()));

    //query mongodb for analyzer definition

    //demultiplexor loop
    loop {
        {
            let demultiplexor = demultiplexor.lock().unwrap();
            if let Err(e) = demultiplexor.fetch() {
                error!("{}", e);
            }
        }

        std::thread::sleep(Duration::new(5, 0))
    }
}
