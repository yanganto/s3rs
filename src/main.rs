extern crate toml;

#[macro_use]
extern crate serde_derive;

extern crate interactor;
use interactor::*;


use std::fs::File;
use std::io::prelude::*;


#[derive(Debug, Deserialize)]
struct CredentialConfig {
    host: String,
    user: Option<String>,
    access_key: String,
    secrete_key: String
}

#[derive(Debug, Deserialize)]
struct Config {
    credential: Option<Vec<CredentialConfig>>,
}


fn main() {

    let mut s3rscfg = std::env::home_dir().unwrap();
    s3rscfg.push(".s3rs");

    let mut f = File::open(s3rscfg).expect("s3rs config file not found");

    let mut config_contents = String::new();
    f.read_to_string(&mut config_contents).expect("s3rs config is not readable");

    let config:Config = toml::from_str(config_contents.as_str()).unwrap();


    let config_option: Vec<String> = config.credential.unwrap().into_iter().map(|x|x.access_key).collect();

    let chosen_int = pick_from_list(None, &config_option, "Selection: ").unwrap();

    println!("credential chosen, you chose '{}'!!", chosen_int);

}
