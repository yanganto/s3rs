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
#[macro_use]
extern crate log;
extern crate md5;
extern crate hmacsha1;
extern crate serde_json;
extern crate regex;


mod handler;

use std::io;
use std::io::{Read, Write, BufReader, BufRead};
use std::fs::{File, OpenOptions};
use std::str;
use std::str::FromStr;
use std::io::stdout;
use log::{Record, Level, Metadata, LevelFilter};
use reqwest::StatusCode;

static MY_LOGGER: MyLogger = MyLogger;

static USAGE:&str = r#"
usage:
    la 
        list all objects

    ls 
        list all buckets

    ls <bucket>
        list all objects of the bucket

    mb <bucket name>
        create bucket

    rm <bucket name>
        delete bucket

    /<uri>?<query string>
        get uri command

    help 
        show this usage

    log <trace/debug/info/error>
        change the log level
        trace for request auth detail
        debug for request header, status code, raw body

    s3_type <aws2/aws4/aws/oss>
        change the auth type for different S3 service

    exit
        quit the programe
"#;

struct MyLogger;

impl log::Log for MyLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}





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

	log::set_logger(&MY_LOGGER).unwrap();
	log::set_max_level(LevelFilter::Error);


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
    debug!("host: {}", credential.host);
    debug!("access key: {}", credential.access_key);
    debug!("secrete key: {}", credential.secrete_key);

    let mut handler = handler::Handler{
        host: &credential.host,
        access_key: &credential.access_key,
        secrete_key: &credential.secrete_key,
        s3_type: handler::S3Type::AWS4 // default use AWS4
    };

    println!("enter command, help for usage or exit for quit");

    let mut raw_input;
    let mut command = String::new(); 

    fn change_s3_type(command: &str, handler: &mut handler::Handler){
        if command.ends_with("aws2"){
            handler.s3_type = handler::S3Type::AWS2;
            println!("using aws version 2 protocol ");
        } else if command.ends_with("aws4") || command.ends_with("aws") ||
             command.ends_with("ceph") {
            handler.s3_type = handler::S3Type::AWS4;
            println!("using aws verion 4 protocol ");
        }else{
            println!("usage: s3type [aws/aws4/aws2/ceph]");
        }
    }

    fn change_log_type(command: &str) {
        if command.ends_with("trace") {
            log::set_max_level(LevelFilter::Trace);
            println!("set up log level trace");
        } else if command.ends_with("debug") {
            log::set_max_level(LevelFilter::Debug);
            println!("set up log level debug");
        } else if command.ends_with("info") {
            log::set_max_level(LevelFilter::Info);
            println!("set up log level info");
        }else{
            println!("usage: log [trace/debug/info/error]");
        }
    }

    while command != "exit" {
        print!("s3rs> ");
        stdout().flush().expect("Could not flush stdout");

        raw_input = read_from_tty(|_buf, b, tty| {
            tty.write(&[b]).expect("Could not write tty");
        }, false, false).unwrap();
        command = String::from_utf8(raw_input).unwrap();
        println!("");
        debug!("===== do command: {:?} =====", command);
        if command.starts_with("la"){
            handler.la();
        } else if command.starts_with("ls"){
            handler.ls(command.split_whitespace().nth(1));
        } else if command.starts_with("mb"){
            handler.mb(command.split_whitespace().nth(1).unwrap());
        } else if command.starts_with("rb"){
            handler.rb(command.split_whitespace().nth(1).unwrap());
        } else if command.starts_with("/"){
            handler.url_command(&command);
        } else if command.starts_with("s3type"){
            change_s3_type(&command, &mut handler);
        } else if command.starts_with("log"){ // XXX this should be better
            change_log_type(&command);
        } else if command.starts_with("exit"){
            println!("Thanks for using, cya~");
        } else if command.starts_with("help"){
            println!("{}", USAGE);
        } else {
            println!("command {} not found, help for usage or exit for quit", command);
        }
        println!("");
        stdout().flush().expect("Could not flush stdout");
    }
}
