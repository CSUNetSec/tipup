#[macro_use(bson, doc)]
extern crate bson;
extern crate dbscan;
extern crate docopt;
extern crate mongodb;
extern crate rustc_serialize;

use bson::Bson;
use bson::ordered::OrderedDocument;
use docopt::Docopt;
use mongodb::{Client, ClientOptions, ThreadedClient};
use mongodb::db::ThreadedDatabase;

use std::fs::File;

const USAGE: &'static str = "
chimpanzee

Usage:
    chimpanzee cluster <filename>
    chimpanzee dump [--ca=<ca-file>] [--cert=<cert-file>] [--key=<key-file>] <mongodb-ip> [--mongodb-port=<mongodb-port>] <username> <password> <min-ts> <max-ts>
    chimpanzee extract <filename> <field>
    chimpanzee (-h | --help)
    chimpanzee --version

Options:
    -h --help                           Show this screen.
    --version                           Show version.
    --ca=<ca-file>                      Ca ssl certification.
    --cert=<cert-file>                  Ssl certification file.
    --key=<key-file>                    Ssl key file.
    --mongodb-port=<mongodb-port>       Mongodb port [default: 27017].
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_field: Option<String>,
    arg_filename: Option<String>,
    arg_max_ts: Option<u64>,
    arg_min_ts: Option<u64>,
    arg_mongodb_ip: Option<String>,
    arg_password: Option<String>,
    arg_username: Option<String>,
    cmd_cluster: bool,
    cmd_dump: bool,
    cmd_extract: bool,
    flag_ca: Option<String>,
    flag_cert: Option<String>,
    flag_key: Option<String>,
    flag_mongodb_port: u16,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
            .and_then(|d| d.decode())
            .unwrap_or_else(|e| e.exit());

    if args.cmd_cluster {
        //read flags into flag vec
        let mut measurements = read_measurements(&args.arg_filename.unwrap());
 
        /*//perform dbscan
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
        //connect to db
        let client = if args.flag_ca.is_some() && args.flag_cert.is_some() && args.flag_key.is_some() {
            let client_options = ClientOptions::with_ssl(&args.flag_ca.unwrap(), 
                    &args.flag_cert.unwrap(), &args.flag_key.unwrap(), true);
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
        let timestamp_document = doc!("$gte" => min_ts, "$lte" => max_ts);
        let exists_document = doc!("$exists" => true);
        let search_document = Some(doc!("timestamp" => timestamp_document, "remaining_attempts" => 0, "measurement_error_message" => exists_document));

        //iterate over results
        let cursor = match db.collection("measurements").find(search_document, None) {
            Ok(cursor) => cursor,
            Err(e) => panic!("{}", e),
        };

        let mut file = match File::create("chimpanzee.bin") {
            Ok(file) => file,
            Err(e) => panic!("{}", e),
        };

        let mut count = 0;
        for document in cursor {
            match document {
                Ok(document) => {
                    count += 1;
                    //print bson object to stream
                    if let Err(e) = bson::encode_document(&mut file, &document) {
                        panic!("{}", e);
                    }
                },
                Err(e) => panic!("{}", e),
            };
        }
        println!("count:{}", count);
    } else if args.cmd_extract {
        //read flags into flag vec
        let measurements = match read_measurements(&args.arg_filename.unwrap()) {
            Ok(measurements) => measurements,
            Err(e) => panic!("{}", e),
        };

        let field = args.arg_field.unwrap();
        for measurement in measurements {
            match measurement.get(&field) {
                Some(&Bson::String(ref value)) => println!("{}", value),
                Some(&Bson::I64(value)) => println!("{}", value),
                Some(&Bson::I32(value)) => println!("{}", value),
                _ => continue,
            }
        }
    }
}

fn read_measurements(filename: &str) -> Result<Vec<OrderedDocument>, std::io::Error> {
    let mut file = match File::open(filename) {
        Ok(file) => file,
        Err(e) => panic!("{}", e),
    };

    let mut measurements = Vec::new();
    loop {
        match bson::decode_document(&mut file) {
            Ok(document) => {
                measurements.push(document);
            },
            Err(e) => {
                println!("ERROR: {}", e);
                break;
            }
        }
    }

    Ok(measurements)
}
