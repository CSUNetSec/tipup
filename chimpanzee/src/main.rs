#[macro_use(bson, doc)]
extern crate bson;
extern crate dbscan;
extern crate docopt;
extern crate mongodb;
extern crate rustc_serialize;

use bson::Bson;
use docopt::Docopt;
use mongodb::{Client, ClientOptions, ThreadedClient};
use mongodb::db::ThreadedDatabase;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

const USAGE: &'static str = "
chimpanzee

Usage:
    chimpanzee cluster <filename>
    chimpanzee dump [--ca=<ca_file>] [--cert=<cert_file>] [--key=<key_file>] <mongodb_ip> [--mongodb_port=<mongodb_port>] <username> <password> <min_ts> <max_ts>
    chimpanzee (-h | --help)
    chimpanzee --version

Options:
    -h --help                           Show this screen.
    --version                           Show version.
    --ca=<ca_file>                      Ca ssl certification.
    --cert=<cert_file>                  Ssl certification file.
    --key=<key_file>                    Ssl key file.
    --mongodb_port=<mongodb_port>       Mongodb port.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_filename: Option<String>,
    arg_password: Option<String>,
    arg_max_ts: Option<u64>,
    arg_min_ts: Option<u64>,
    arg_mongodb_ip: Option<String>,
    arg_username: Option<String>,
    cmd_cluster: bool,
    cmd_dump: bool,
    flag_ca_file: Option<String>,
    flag_cert_file: Option<String>,
    flag_key_file: Option<String>,
    flag_mongodb_port: u16,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
            .and_then(|d| d.decode())
            .unwrap_or_else(|e| e.exit());

    if args.cmd_cluster {
        /*//read flags into flag vec
        let mut measurements = Vec::new();
        {
            let file = match File::open(filename) {
                Ok(file) => file,
                Err(e) => panic!("{}", e),
            };

            let mut buf_reader = BufReader::new(args.arg_filename.unwrap());
            let mut line = String::new();
            loop {
                if let Err(e) = buf_reader.read_line(&mut line) {
                    panic!("{}", e);
                }

                if line.len() == 0 {
                    break;
                }

                match serde_json::from_str::<Value>(&line) {
                    Ok(json) => measurements.push(json),
                    Err(e) => panic!("{}", e);
                }

                line.clear();
            }
        }
 
        //perform dbscan
        let (clustering, matrix) = cluster::dbscan(&flags, 65.0, 3);

        //print graphml
        println!("<graphml>");
        println!("\t<key id=\"d0\" for=\"node\" attr.name=\"Modularity Class\" attr.type=\"integer\"/>");
        println!("\t<key id=\"d1\" for=\"edge\" attr.name=\"Weight\" attr.type=\"double\"/>");
        println!("\t<graph id=\"F\" edgedefault=\"undirected\">");
        for i in 0..flags.len() {
            println!("\t\t<node id=\"{}\">", flags[i].id);
            match clustering[i] {
                Some(cluster) => println!("\t\t\t<data key=\"d0\">{}</data>", cluster),
                None => println!("\t\t\t<data key=\"d0\">-1</data>"),
            }
            println!("\t\t</node>");
        }

        for i in 0..matrix.size() {
            for j in i+1..matrix.size() {
                let distance = matrix.get(i, j);
                if distance == 100.0 {
                    continue;
                }

                println!("\t\t<edge source=\"{}\" target=\"{}\">", flags[i].id, flags[j].id);

                let weight = (-9.0 / 100.0) * distance + 10.0;
                println!("\t\t\t<data key=\"d1\">{}</data>", weight);
                println!("\t\t</edge>");
            }
        }
        println!("\t</graph>");
        println!("</graphml>");*/
    } else if args.cmd_dump {
        //connect to tipup db
        let client = if args.flag_ca_file.is_some() && args.flag_cert_file.is_some() && args.flag_key_file.is_some() {
            let client_options = ClientOptions::with_ssl(&args.flag_ca_file.unwrap(), 
                    &args.flag_cert_file.unwrap(), &args.flag_key_file.unwrap(), true);
            Client::connect_with_options(&args.arg_mongodb_ip.unwrap(), args.flag_mongodb_port, client_options)
        } else {
            Client::connect(&args.arg_mongodb_ip.unwrap(), args.flag_mongodb_port)
        };

        let db = match client {
            Ok(client) => {
                let db = client.db("proddle");
                if let Err(e) = db.auth(&args.arg_username.unwrap(), &args.arg_password.unwrap()) {
                    panic!("{}", e);
                }
                db
            },
            Err(e) => panic!("{}", e),
        };
     
        //query proddle db for recent flags
        let (min_ts, max_ts) = (args.arg_min_ts.unwrap(), args.arg_max_ts.unwrap());
        let timestamp_gte_document = doc!("$gte" => min_ts);
        let timestamp_lte_document = doc!("$lt" => max_ts);
        let search_document = Some(doc!("timestamp" => timestamp_gte_document, "timestamp" => timestamp_lte_document));

        //iterate over results
        let cursor = match db.collection("measurements").find(search_document, None) {
            Ok(cursor) => cursor,
            Err(e) => panic!("{}", e),
        };

        let stdout = io::stdout();
        let mut handle = stdout.lock();
        for document in cursor {
            let document = match document {
                Ok(document) => document,
                Err(e) => panic!("{}", e),
            };
            
            //check if measurement is a failure
            if let Some(_) = document.get("error_message") {
                match document.get("remaining_attempts") {
                    Some(&Bson::I32(0)) => {},
                    Some(&Bson::I64(0)) => {},
                    _ => continue,
                }
            }

            //print bson object to stream
            if let Err(e) = bson::encode_document(&mut handle, &document) {
                panic!("{}", e);
            }
        }
    }
}
