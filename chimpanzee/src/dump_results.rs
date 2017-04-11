use bson::{self, Bson};
use mongodb::{Client, ClientOptions, ThreadedClient};
use mongodb::db::ThreadedDatabase;
use serde_json::value::ToJson;

use Flag;

pub fn dump_results(minimum_timestamp: i64, maximum_timestamp: i64) {
    let ca_file = "/etc/ssl/cacert.pem";
    let certificate_file = "/etc/ssl/generated-certs/proddle.crt";
    let key_file = "/etc/ssl/generated-certs/proddle.key";
    let mongodb_ip_address = "mongo1.netsec.colostate.edu";
    let mongodb_port = 27017;
    let username = "tipup";
    let password = "4a]p[22LXs<B(muG+4HE";

    //connect to tipup db
    let client_options = ClientOptions::with_ssl(ca_file, certificate_file, key_file, true);
    let client = match Client::connect_with_options(mongodb_ip_address, mongodb_port, client_options)  {
        Ok(client) => client,
        Err(e) => panic!("{}", e),
    };

    let tipup_db = client.db("tipup");
    if let Err(e) = tipup_db.auth(username, password) {
        panic!("{}", e);
    }
 
    //query tipup db for recent flags
    let timestamp_gte_document = doc!("$gte" => minimum_timestamp);
    let timestamp_lte_document = doc!("$lt" => maximum_timestamp);
    let search_document = Some(doc!("timestamp" => timestamp_gte_document, "timestamp" => timestamp_lte_document));

    //iterate over results
    let cursor = match tipup_db.collection("flags").find(search_document, None) {
        Ok(cursor) => cursor,
        Err(e) => panic!("{}", e),
    };

    for document in cursor {
        let document = match document {
            Ok(document) => document,
            Err(e) => panic!("{}", e),
        };
        
        let flag: Flag = match bson::from_bson(Bson::Document(document)) {
            Ok(flag) => flag,
            Err(_) => panic!("failed to parse bson document into flag"),
        };

        println!("{}", flag.to_json().unwrap());
    }
}
