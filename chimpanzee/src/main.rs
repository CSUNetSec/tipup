extern crate dbscan;
#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;
extern crate rustc_serialize;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use bson::oid::ObjectId;

mod cluster;
mod dump_results;

use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, Deserialize, Serialize)]
pub enum FlagStatus {
    Unreachable,
    Warning,
    Internal,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Flag {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub measurement_id: ObjectId,
    pub timestamp: i64,
    pub hostname: String,
    pub ip_address: String,
    pub domain: String,
    pub domain_ip_address: Option<String>,
    pub url: String,
    pub status: FlagStatus,
    pub analyzer: String,
}

fn compute_flag_distance(flag_one: &Flag, flag_two: &Flag) -> f64 {
    //domain
    if !flag_one.domain.eq(&flag_two.domain) {
        return 100.0;
    }

    //timestamp
    let timestamp_difference = (flag_one.timestamp - flag_two.timestamp).abs() as f64 / 3600.0; //difference in hours
    let timestamp_score = match timestamp_difference {
        0.0 ... 1.0 => (timestamp_difference + 1.00027).log2() / (24.0 as f64).log2(), //logarithmic ratio
        _ => 1.0,
    };

    //urls - TODO fuzzy match
    /*let url_score = match flag_one.url.eq(&flag_two.url) {
        true => 0.0,
        false => 1.0,
    };*/

    return timestamp_score * 100.0;
}

fn main() {
    //dump flags from mongodb
    //dump_results::dump_results(1488672000, 1489968000); //2017.03.05 - 2017.03.20

    //perform dbscan
    let flags = read_data("2017.03.05-2017.03.20.csv");
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
    println!("</graphml>");
}

fn read_data(filename: &str) -> Vec<Flag> {
    //read flags into flag vec
    let file = match File::open(filename) {
        Ok(file) => file,
        Err(e) => panic!("{}", e),
    };

    let mut buf_reader = BufReader::new(file);
    let mut line = String::new();
    let mut flags = Vec::new();
    loop {
        if let Err(e) = buf_reader.read_line(&mut line) {
            panic!("{}", e);
        }

        if line.len() == 0 {
            break;
        }

        let flag: Flag = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(e) => panic!("{}", e),
        };

        flags.push(flag);
        line.clear();
    }

    flags
}
