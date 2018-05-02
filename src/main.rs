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


mod handler;

use std::io;
use std::io::{Read, Write, BufReader, BufRead};
use std::fs::{File, OpenOptions};
use std::str;
use std::str::FromStr;
use std::io::stdout;
use log::{Record, Level, Metadata, LevelFilter};

static MY_LOGGER: MyLogger = MyLogger;

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

    let handler = handler::Handler{
        host: &credential.host,
        access_key: &credential.access_key,
        secrete_key: &credential.secrete_key,
        s3_type: handler::S3Type::AWS4 // current only AWS4 implement
    };

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
        debug!("===== do command: {:?} =====", command);
        if command.starts_with("la"){
            res = handler.la();

            println!("Status: {}", res.status());
            println!("Headers:\n{}", res.headers());

            // copy the response body directly to stdout
            let _ = std::io::copy(&mut res, &mut std::io::stdout()).unwrap();
        } else if command.starts_with("/"){
            res = handler.url_command(&command);

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
