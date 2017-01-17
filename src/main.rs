#[macro_use]
extern crate clap;

use clap::App;

fn main() {
    //parse arguments
    let yaml = load_yaml!("main_args.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let mongodb_ip_address = matches.value_of("MONGODB_IP_ADDRESS").unwrap();
    let mongodb_port = match matches.value_of("MONGODB_PORT").unwrap().parse::<u16>() {
        Ok(mongodb_port) => mongodb_port,
        Err(e) => panic!("failed to parse monogodb_port as u16: {}", e);
    };

    //start mongodb client
    let client = match Client::connect(mongodb_ip_address, mongodb_port) {
        Ok(client) => client,
        Err(e) => panic!("failed to connect to mongodb cluster: {}", e);
    };

    println!("{:?}", matches);
}
