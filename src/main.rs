#[macro_use(bson, doc)]
extern crate bson;
#[macro_use]
extern crate clap;
extern crate mongodb;
extern crate time;

use bson::Bson;
use bson::ordered::OrderedDocument;
use clap::App;
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase;

fn main() {
    //parse arguments
    let yaml = load_yaml!("main_args.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let mongodb_ip_address = matches.value_of("MONGODB_IP_ADDRESS").unwrap();
    let mongodb_port = match matches.value_of("MONGODB_PORT").unwrap().parse::<u16>() {
        Ok(mongodb_port) => mongodb_port,
        Err(e) => panic!("failed to parse monogodb_port as u16: {}", e),
    };

    //start mongodb client
    let client = match Client::connect(mongodb_ip_address, mongodb_port) {
        Ok(client) => client,
        Err(e) => panic!("failed to connect to mongodb cluster: {}", e),
    };

    //create document
    let now = time::now_utc().to_timespec().sec;
    let yesterday = now - (24 * 60 * 60);

    let mut report = OrderedDocument::new();
    report.insert_bson("Timestamp".to_owned(), Bson::I64(now));
    let mut vantages = Vec::new();

    //query for unique hostnames
    let collection = client.db("proddle").collection("results");
    let results = match collection.distinct("Hostname", Some(doc!{"Timestamp" => {"$gte" => yesterday}}), None) {
        Ok(results) => results,
        Err(e) => panic!("failed to execute distinct hostname query: {}", e),
    };

    //iterate over unique hostnames
    for result in results {
        let hostname = match result {
            Bson::String(hostname) => hostname,
            _ => continue,
        };

        let mut vantage = OrderedDocument::new();
        vantage.insert_bson("Hostname".to_owned(), Bson::String(hostname.to_owned()));

        //get ip address of hostname
        let filter_hostname = hostname.to_owned();
        let filter_document = Some(doc!{"Timestamp" => {"$gte" => yesterday}, "Hostname" => filter_hostname});
        let ip_results = match collection.distinct("IpAddress", filter_document, None) {
            Ok(ip_results) => ip_results,
            Err(e) => {
                println!("unable to retrieve ip address for host '{}': {}", hostname, e);
                Vec::new()
            },
        };

        //iterate over ip addresses
        let mut ip_addresses = Vec::new();
        for ip_result in ip_results {
            let ip_address = match ip_result {
                Bson::String(ip_address) => ip_address,
                _ => continue,
            };

            //create ip document
            let mut ip = OrderedDocument::new();
            ip.insert_bson("Ip".to_owned(), Bson::String(ip_address.to_owned()));

            ip_addresses.push(Bson::Document(ip));
        }

        vantage.insert_bson("IpAddresses".to_owned(), Bson::Array(ip_addresses));
 
        //get all modules for this vantage
        let filter_hostname = hostname.to_owned();
        let filter_document = Some(doc!{"Timestamp" => {"$gte" => yesterday}, "Hostname" => filter_hostname});
        let module_results = match collection.distinct("Module", filter_document, None) {
            Ok(module_results) => module_results,
            Err(e) => {
                println!("unable to retrieve modules for host '{}': {}", hostname, e);
                Vec::new()
            },
        };

        //itereate over unique modules
        let mut modules = Vec::new();
        for module_result in module_results {
            let module_name = match module_result {
                Bson::String(module_name) => module_name,
                _ => continue,
            };

            //create module document
            let mut module = OrderedDocument::new();
            module.insert_bson("Name".to_owned(), Bson::String(module_name.to_owned()));

            //get count of modules
            let (filter_hostname, filter_module_name) = (hostname.to_owned(), module_name.to_owned());
            let filter = Some(doc!{"Timestamp" => {"$gte" => yesterday}, "Hostname" => filter_hostname, "Module" => filter_module_name});
            let module_count = match collection.count(filter, None) {
                Ok(module_count) => module_count,
                Err(e) => {
                    println!("unable to retrieve count for module '{}' and host '{}': {}", module_name, hostname, e);
                    -1
                },
            };

            module.insert_bson("Count".to_owned(), Bson::I64(module_count));
            modules.push(Bson::Document(module));
        }

        vantage.insert_bson("Modules".to_owned(), Bson::Array(modules));
        vantages.push(Bson::Document(vantage));
    }

    report.insert_bson("Vantages".to_owned(), Bson::Array(vantages));

    let collection = client.db("proddle").collection("vantage_reports");
    if let Err(e) = collection.insert_one(report, None) {
        panic!("failed to insert report: {}", e);
    }
}
