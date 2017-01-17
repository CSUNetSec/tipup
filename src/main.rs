#[macro_use]
extern crate clap;

use clap::App;

fn main() {
    //parse arguments
    let yaml = load_yaml!("main_args.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    println!("{:?}", matches);
}
