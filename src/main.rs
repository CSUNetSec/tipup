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
use mongodb::{Client, ClientOptions, ThreadedClient};
use mongodb::db::ThreadedDatabase;

mod analyzer;
mod demultiplexor;
mod error;

use error::TipupError;
use demultiplexor::Demultiplexor;

use std::sync::{Arc, Mutex};
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
    let yaml = load_yaml!("main_args.yaml");
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

    let db = client.db("proddle");
    if let Err(e) = db.auth(&username, &password) {
        panic!("{}", e);
    }

    //query mongodb for analyzer definition

    //create new demultiplexor
    let mut demultiplexor = Arc::new(Mutex::new(Demultiplexor::new(db)));

    //demultiplexor loop
    loop {
        {
            let demultiplexor = demultiplexor.lock().unwrap();
            if let Err(e) = demultiplexor.fetch() {
                error!("{}", e);
            }
        }

        std::thread::sleep(Duration::new(30, 0))
    }
}
