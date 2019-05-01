extern crate toml;
#[macro_use]
extern crate serde_derive;
extern crate interactor; 
extern crate reqwest;

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
extern crate quick_xml;
extern crate colored;
extern crate s3handler;


use std::io;
use std::io::{Read, Write, BufReader, BufRead};
use std::fs::{File, OpenOptions};
use std::str;
use std::str::FromStr;
use std::io::stdout;
use log::{Record, Level, Metadata, LevelFilter};
use colored::*;
use regex::Regex;

static MY_LOGGER: MyLogger = MyLogger;
static S3_FORMAT: &'static str = r#"[sS]3://(?P<bucket>[A-Za-z0-9\-\._]+)(?P<object>[A-Za-z0-9\-\._/]*)"#;

struct MyLogger;

impl log::Log for MyLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            match record.level() {
                log::Level::Error => println!("{} - {}", "ERROR".red().bold(), record.args()),
                log::Level::Warn => println!("{} - {}", "WARN".red(), record.args()),
                log::Level::Info => println!("{} - {}", "INFO".cyan(), record.args()),
                log::Level::Debug => println!("{} - {}", "DEBUG".blue().bold(), record.args()),
                log::Level::Trace => println!("{} - {}", "TRACE".blue(), record.args())
            }
            
        }
    }
    fn flush(&self) {}
}


#[derive(Debug, Deserialize)]
struct Config {
    credential: Option<Vec<s3handler::CredentialConfig>>
}
impl <'a> Config {
    fn gen_selecitons(&'a self) -> Vec<String>{
        let mut display_list = Vec::new();
        let credential = &self.credential.clone().unwrap();
        for cre in credential.into_iter(){
            let c = cre.clone();
            let option = String::from(format!("[{}] {} ({}) {} ({})", 
                                                  c.s3_type.unwrap_or(String::from("aws")), 
                                                  c.host, 
                                                  c.region.unwrap_or(String::from("us-east-1")), 
                                                  c.user.unwrap_or(String::from("user")), 
                                                  c.access_key));
            display_list.push(option);
        }
        display_list
    }
}

fn read_parse<T>(tty: &mut File, prompt: &str, min: T, max: T) 
    -> io::Result<T> where T: FromStr + Ord {

    tty.write_all(prompt.as_bytes());
    let mut reader = io::BufReader::new(tty);
    let mut result = String::new();
    reader.read_line(&mut result);
    match result.replace("\n", "").parse::<T>() {
        Ok(x) => if x >= min && x <= max { Ok(x) } 
                 else { read_parse(reader.into_inner(), prompt, min, max) },
        _ => read_parse(reader.into_inner(), prompt, min, max)
    }
}

fn my_pick_from_list_internal<T: AsRef<str>>(items: &[T], prompt: &str) -> io::Result<usize> {
    let mut tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
    let pad_len = ((items.len() as f32).log10().floor() + 1.0) as usize;
    for (i, item) in items.iter().enumerate() {
        tty.write_all(format!("{1:0$}. {2}\n", pad_len, i + 1, item.as_ref().replace("\n", "")).as_bytes())?
    }
    let idx = read_parse::<usize>(&mut tty, prompt, 1, items.len())? - 1;
    Ok(idx)
}

fn main() {
    log::set_logger(&MY_LOGGER).unwrap();
    log::set_max_level(LevelFilter::Error);

    let mut s3rscfg = std::env::home_dir().unwrap();
    s3rscfg.push(".s3rs.toml");

    let mut f;
    if s3rscfg.exists() {
        f = File::open(s3rscfg).expect("s3rs config file not found");
    } else {
        f = File::create(s3rscfg).expect("Can not write s3rs config file");
        let _ = f.write_all(
            b"[[credential]]\ns3_type = \"aws\"\nhost = \"s3.us-east-1.amazonaws.com\"\nuser = \"admin\"\naccess_key = \"L2D11MY86GEVA6I4DX2S\"\nsecret_key = \"MBCqT90XMUaBcWd1mcUjPPLdFuNZndiBk0amnVVg\"\nregion = \"us-east-1\""
            );
        print!("Config file {} is created in your home folder, please edit it and add your credentials", ".s3rs.toml".bold());
        return 
    }

    let mut config_contents = String::new();
    f.read_to_string(&mut config_contents).expect("s3rs config is not readable");

    let config:Config = toml::from_str(config_contents.as_str()).unwrap();
    let config_option: Vec<String> = config.gen_selecitons();

    let mut chosen_int = my_pick_from_list_internal(&config_option, "Selection: ").unwrap();
    let config_list = config.credential.unwrap();
    let mut handler = s3handler::Handler::init_from_config(&config_list[chosen_int]);
    let mut login_user = config_list[chosen_int].user.clone().unwrap_or("unknown".to_string());
    let mut s3_type = config_list[chosen_int].s3_type.clone().unwrap_or("aws".to_string());

    println!("enter command, help for usage or exit for quit");

    // let mut raw_input;
    let mut command = String::new(); 

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
        } else if command.ends_with("error") {
            log::set_max_level(LevelFilter::Error);
            println!("set up log level error");
        }else{
            println!("usage: log [trace/debug/info/error]");
        }
    }

    fn print_if_error(result: Result<(),&str>) {
        match result{
            Err(e) => println!("{}", e),
            Ok(_) => {}
        };
    }

    while command != "exit" && command != "quit" {
        command = match OpenOptions::new().read(true).write(true).open("/dev/tty") {
            Ok(mut tty) => {
                tty.flush().expect("Could not open tty");
                tty.write_all(
                    format!("{} {} {} ", "s3rs".green(), login_user.cyan(), ">".green()).as_bytes());
                let reader = BufReader::new(&tty);
                let mut command_iter = reader.lines().map(|l| l.unwrap());
                command_iter.next().unwrap_or("logout".to_string())
            },
            Err(e) => {println!("{:?}", e);  "quit".to_string()}
        };

        debug!("===== do command: {} =====", command);
        if command.starts_with("la"){
            print_if_error(handler.la());
        } else if command.starts_with("ls"){
            print_if_error(handler.ls(command.split_whitespace().nth(1)));
        } else if command.starts_with("put"){
            match handler.put(command.split_whitespace().nth(1).unwrap_or(""), command.split_whitespace().nth(2).unwrap_or("")) {
                Err(e) => println!("{}", e),
                Ok(_) => println!("upload completed")
            };
        } else if command.starts_with("get"){
            match handler.get(command.split_whitespace().nth(1).unwrap_or(""), command.split_whitespace().nth(2)){
                Err(e) => println!("{}", e),
                Ok(_) => println!("download completed")
            };
        } else if command.starts_with("cat"){ print_if_error(handler.cat(command.split_whitespace().nth(1).unwrap_or("")));
        } else if command.starts_with("del"){
            match handler.del(command.split_whitespace().nth(1).unwrap_or("")){
                Err(e) => println!("{}", e),
                Ok(_) => println!("deletion completed")
            }
        } else if command.starts_with("tag"){
            let mut iter = command.split_whitespace();
            let action = iter.nth(1).unwrap_or("");
            let target = iter.nth(0).unwrap_or("");
            let mut tags = Vec::new();
            loop {
                match iter.next() {
                    Some(kv_pair) => {
                        match kv_pair.find('=') {
                            Some(_) => {
                                tags.push(
                                    (kv_pair.split('=').nth(0).unwrap(),
                                     kv_pair.split('=').nth(1).unwrap()))},
                            None =>  {tags.push((&kv_pair, ""))}
                        }
                    }
                    None =>{break;}
                };
            }
            match action {
                "add"|"put" => match handler.add_tag(target, &tags){
                    Err(e) => println!("{}", e),
                    Ok(_) => println!("tag completed")
                },
                "del"|"rm" => match handler.del_tag(target){
                    Err(e) => println!("{}", e),
                    Ok(_) => println!("tag removed")
                },
                "ls"|"list" => match handler.list_tag(target){
                    Err(e) => println!("{}", e),
                    Ok(_) => {}
                }
                _ => println!("only support these tag actions: ls, add, put, del, rm")
            }
        } else if command.starts_with("usage"){
            let mut iter = command.split_whitespace();
            let target = iter.nth(1).unwrap_or("");
            let mut options = Vec::new();
            loop {
                match iter.next() {
                    Some(kv_pair) => {
                        match kv_pair.find('=') {
                            Some(_) => {
                                options.push(
                                    (kv_pair.split('=').nth(0).unwrap(),
                                     kv_pair.split('=').nth(1).unwrap()))},
                            None =>  {options.push((&kv_pair, ""))}
                        }
                    }
                    None =>{break;}
                };
            }
            match handler.usage(target, &options) {
                Err(e) => println!("{}", e),
                Ok(_) => {}
            }
        } else if command.starts_with("mb"){
            print_if_error(handler.mb(command.split_whitespace().nth(1).unwrap_or("")));
        } else if command.starts_with("rb"){
            print_if_error(handler.rb(command.split_whitespace().nth(1).unwrap_or("")));
        } else if command.starts_with("/"){
            match handler.url_command(&command){
                Err(e) => println!("{}", e),
                Ok(_) => {}
            };
        } else if command.starts_with("info"){
            let target = command.split_whitespace().nth(1).unwrap_or("");
            let caps;
            let bucket = if target.starts_with("s3://") || target.starts_with("S3://") {
                let re = Regex::new(S3_FORMAT).unwrap();
                caps = re.captures(command.split_whitespace().nth(1).unwrap_or(""))
                    .expect("S3 object format error.");
                &caps["bucket"]
            } else {
                target
            };
            println!("{}", "location:".yellow().bold());
            let _ = handler.url_command(format!("/{}?location", bucket).as_str());
            println!("\n{}", "acl:".yellow().bold());
            let _ = handler.url_command(format!("/{}?acl", bucket).as_str());
            println!("\n{}", "versioning:".yellow().bold());
            let _ = handler.url_command(format!("/{}?versioning", bucket).as_str());
            match s3_type.as_str() {
                "ceph" => {
                    println!("\n{}", "version:".yellow().bold());
                    let _ = handler.url_command(format!("/{}?version", bucket).as_str());
                    println!("\n{}", "uploads:".yellow().bold());
                    let _ = handler.url_command(format!("/{}?uploads", bucket).as_str());
                },
                "aws" | _ => {}
            }
        } else if command.starts_with("s3_type"){
            handler.change_s3_type(&command);
        } else if command.starts_with("auth_type"){
            handler.change_auth_type(&command);
        } else if command.starts_with("format"){
            handler.change_format_type(&command);
        } else if command.starts_with("url_style"){
            handler.change_url_style(&command);
        } else if command.starts_with("logout"){ 
            println!("");
            chosen_int = my_pick_from_list_internal(&config_option, "Selection: ").unwrap();
            handler = s3handler::Handler::init_from_config(&config_list[chosen_int]);
            login_user = config_list[chosen_int].user.clone().unwrap_or(" ".to_string());
            s3_type = config_list[chosen_int].s3_type.clone().unwrap_or("aws".to_string());
        } else if command.starts_with("log"){ 
            change_log_type(&command);
        } else if command.starts_with("exit") || command.starts_with("quit") {
            println!("Thanks for using, cya~");
        } else if command.starts_with("help"){
            println!(r#"
USAGE:

    {0}
        list all objects
    
    {1}
        list all buckets

    {1} {2}
        list all objects of the bucket

    {3} {2}
        create bucket

    {4} {2}
        delete bucket

    {5} {6} s3://{2}/{7}
        upload the file with specify object name

    {5} {6} s3://{2}
        upload the file as the same file name
        
    {5} test s3://{2}/{7}
        upload a small test text file with specify object name

    {8} s3://{2}/{7} {6}
        download the object

    {8} s3://{2}/{7} 
        download the object to current folder

    {9} s3://{2}/{7} 
        display the object content

    {10} s3://{2}/{7} 
        delete the object

    {29} {1}/{36} s3://{2}/{7}
        list tags of the object

    {29} {33}/{5} s3://{2}/{7}  {30}={31} ...
        add tags to the object

    {29} {10}/{4} s3://{2}/{7}
        remove tags from the object

    /{11}?{12}
        get uri command

    {13}
        show this usage

    {14} {32}/{15}/{16}/{17}/{18}
        change the log level
        {32} for every thing
        {15} for request auth detail
        {16} for request header, status code, raw body
        {17} for request http response
        {18} is default

    {19} {20}/{21}
        change the auth type and format for different S3 service

    {22} {23}/{24}
        change the auth type 

    {25} {26}/{27}
        change the request format

    {28}
        quit the programe

    {34} / {35}
        logout and reselect account

    {37} s3://{2} 
        show the usage of the bucket (ceph admin only)

    {38} s3://{2} / {38} {2}
        show the bucket information
        acl(ceph, aws), location(ceph, aws), versioning(ceph, aws), uploads(ceph), version(ceph)

    If you have any issue, please submit to here https://github.com/yanganto/s3rs/issues 
        "#, 
            "la".bold(), "ls".bold(), "<bucket>".cyan(), "mb".bold(), "rm".bold(),
            "put".bold(), "<file>".cyan(), "<object>".cyan(), "get".bold(), "cat".bold(),
            "del".bold(), "<uri>".cyan(), "<query string>".cyan(), "help".bold(), "log".bold(),
            "trace".blue(), "debug".blue(), "info".blue(), "error".blue(), "s3_type".bold(),
            "aws".blue(), "ceph".blue(), "auth_type".bold(), "aws2".blue(), "aws4".blue(),
            "format".bold(), "xml".blue(), "json".blue(), "exit".bold(), "tag".bold(),
            "<key>".cyan(), "<value>".cyan(), "trace".blue(), "add".bold(), "logout".bold(), 
            "Ctrl + d".bold(), "list".bold(), "usage".bold(), "info".bold()//38
            ); 
        } else {
            println!("command {} not found, help for usage or exit for quit", command);
        }
        println!("");
        stdout().flush().expect("Could not flush stdout");
    }
}
