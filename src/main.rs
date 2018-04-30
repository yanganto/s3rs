extern crate toml;
#[macro_use]
extern crate serde_derive;
extern crate interactor; 
use interactor::read_from_tty; 
extern crate reqwest;
#[macro_use] 
extern crate hyper;
extern crate chrono;
extern crate hmac;
extern crate sha2;
extern crate base64;
extern crate crypto;
extern crate rustc_serialize;
extern crate url;


pub mod aws;

use std::io;
use std::io::{Read, Write, BufReader, BufRead};
use std::fs::{File, OpenOptions};
use std::str;
use std::str::FromStr;
use std::io::stdout;
use reqwest::header;
use chrono::prelude::*;




#[derive(Debug, Clone, Deserialize)]
struct CredentialConfig {
    host: String,
    user: Option<String>,
    access_key: String,
    secrete_key: String
}

#[derive(Debug, Deserialize)]
struct Config {
    credential: Option<Vec<CredentialConfig>>
}
impl <'a> Config {
    fn gen_selecitons(&'a self) -> Vec<String>{
        let mut display_list = Vec::new();
        let credential = &self.credential.clone().unwrap();
        for cre in credential.into_iter(){
            let c = cre.clone();
            let mut option = String::from(format!("{} {} ({})", c.host, c.user.unwrap_or(String::from("")), c.access_key));
            display_list.push(option);
        }
        display_list
    }
}


fn read_parse<T>(tty: &mut File, prompt: &str, min: T, max: T) -> io::Result<T> where T: FromStr + Ord {
    try!(tty.write_all(prompt.as_bytes()));
    let mut reader = BufReader::new(tty);
    let mut result = String::new();
    try!(reader.read_line(&mut result));
    match result.replace("\n", "").parse::<T>() {
        Ok(x) => if x >= min && x <= max { Ok(x) } else { read_parse(reader.into_inner(), prompt, min, max) },
        _ => read_parse(reader.into_inner(), prompt, min, max)
    }
}

fn my_pick_from_list_internal<T: AsRef<str>>(items: &[T], prompt: &str) -> io::Result<usize> {
    let mut tty = try!(OpenOptions::new().read(true).write(true).open("/dev/tty"));
    let pad_len = ((items.len() as f32).log10().floor() + 1.0) as usize;
    for (i, item) in items.iter().enumerate() {
        try!(tty.write_all(format!("{1:0$}. {2}\n", pad_len, i + 1, item.as_ref().replace("\n", "")).as_bytes()))
    }
    let idx = try!(read_parse::<usize>(&mut tty, prompt, 1, items.len())) - 1;
    Ok(idx)
}

		
fn main() {

    let verbose = true;

    let mut s3rscfg = std::env::home_dir().unwrap();
    s3rscfg.push(".s3rs");

    let mut f = File::open(s3rscfg).expect("s3rs config file not found");

    let mut config_contents = String::new();
    f.read_to_string(&mut config_contents).expect("s3rs config is not readable");

    let config:Config = toml::from_str(config_contents.as_str()).unwrap();


    let config_option: Vec<String> = config.gen_selecitons();

    let chosen_int = my_pick_from_list_internal(&config_option, "Selection: ").unwrap();


    // save the credential user this time 
    let credential = &config.credential.unwrap()[chosen_int];
    if verbose {
        println!("host: {}", credential.host);
        println!("access key: {}", credential.access_key);
        println!("secrete key: {}", credential.secrete_key);
    }

    println!("enter command, help for usage or exit for quit");

    let mut raw_input;
    let mut command = String::new(); 
    let mut res;
    while command != "exit" {
        print!("> ");
        stdout().flush().expect("Could not flush stdout");

        raw_input = read_from_tty(|_buf, b, tty| {
            tty.write(&[b]).expect("Could not write tty");
        }, false, false).unwrap();
        command = String::from_utf8(raw_input).unwrap();
        println!("");
        if verbose {
            println!("===== do command: {:?} =====", command);
        }
        if command.starts_with("la"){


            // XXX: Implement AWS4 auth here
            let mut query = String::from_str("http://").unwrap();
            query.push_str(credential.host.as_str());
            // query.push_str("www.rust-lang.org");
            // query.push_str("?format=json");

            let mut headers = header::Headers::new();

            let utc: DateTime<Utc> = Utc::now();   
            // let utc: DateTime<Utc> = Utc.ymd(2018, 04, 29).and_hms(4, 45, 1);
            header! { (XAMZDate, "x-amz-date") => [String] }
            headers.set(XAMZDate(utc.to_rfc2822()));


            // let mut mac = Hmac::<Sha256>::new(
            //         &base64::decode(credential.secrete_key.as_str()).expect("secrete key decode error")
            //     ).unwrap();
            // mac.input(b"");
            // let result = mac.result();
            // let code_bytes = result.code();

            // println!("auth: {:?}", base64::encode(&code_bytes));

            let mut authorize_string = String::from_str("AWS ").unwrap();
            authorize_string.push_str(credential.access_key.as_str());
            authorize_string.push(':');
            // authorize_string.push_str(base64::encode(&code_bytes).as_str());
            // headers.set(header::Authorization(authorize_string));


            // get a client builder
            let client = reqwest::Client::builder()
                .default_headers(headers)
                .build().unwrap();

            res = client.get(query.as_str()).send().unwrap();

            println!("Status: {}", res.status());
            println!("Headers:\n{}", res.headers());

            // copy the response body directly to stdout
            let _ = std::io::copy(&mut res, &mut std::io::stdout()).unwrap();
        } else if command.starts_with("exit"){
            println!("Thanks for using, cya~");
        } else {
            println!("command {} not found, help for usage or exit for quit", command);
        }
        println!("");
        stdout().flush().expect("Could not flush stdout");
    }
}
