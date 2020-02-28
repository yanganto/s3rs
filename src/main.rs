extern crate dirs;
extern crate toml;
#[macro_use]
extern crate serde_derive;
extern crate interactor;
extern crate reqwest;

extern crate base64;
extern crate chrono;
#[macro_use]
extern crate clap;
extern crate crypto;
extern crate hmac;
extern crate hyper;
extern crate rustc_serialize;
extern crate sha2;
extern crate url;
#[macro_use]
extern crate log;
extern crate colored;
extern crate hmacsha1;
extern crate md5;
extern crate quick_xml;
extern crate regex;
extern crate s3handler;
extern crate serde_json;

use clap::{App, Arg};
use colored::*;
use dirs::home_dir;
use log::{Level, LevelFilter, Metadata, Record};
use regex::Regex;
use std::fs::{create_dir, read_dir, File, OpenOptions};
use std::io;
use std::io::stdout;
use std::io::{BufRead, BufReader, Read, Write};
use std::str;
use std::str::FromStr;
use hyper::rt::Future;
use ipfs_api::IpfsClient;
use std::io::Cursor;

static MY_LOGGER: MyLogger = MyLogger;
static S3_FORMAT: &'static str =
    r#"[sS]3://(?P<bucket>[A-Za-z0-9\-\._]+)(?P<object>[A-Za-z0-9\-\._/]*)"#;
static S3RS_CONFIG_FOLDER: &'static str = ".config/s3rs";

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
                log::Level::Trace => println!("{} - {}", "TRACE".blue(), record.args()),
            }
        }
    }
    fn flush(&self) {}
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
    } else if command.ends_with("error") {
        log::set_max_level(LevelFilter::Error);
        println!("set up log level error");
    } else {
        println!("usage: log [trace/debug/info/error]");
    }
}

#[derive(Debug, Deserialize)]
struct Config {
    credential: Option<Vec<s3handler::CredentialConfig>>,
}
impl<'a> Config {
    fn gen_selecitons(&'a self) -> Vec<String> {
        let mut display_list = Vec::new();
        let credential = &self.credential.clone().unwrap();
        for cre in credential.into_iter() {
            let c = cre.clone();
            let option = String::from(format!(
                "[{}] {} ({}) {} ({})",
                c.s3_type.unwrap_or(String::from("aws")),
                c.host,
                c.region.unwrap_or(String::from("us-east-1")),
                c.user.unwrap_or(String::from("user")),
                c.access_key
            ));
            display_list.push(option);
        }
        display_list
    }
}

fn read_parse<T>(tty: &mut File, prompt: &str, min: T, max: T) -> io::Result<T>
where
    T: FromStr + Ord,
{
    let _ = tty.write_all(prompt.as_bytes());
    let mut reader = io::BufReader::new(tty);
    let mut result = String::new();
    let _ = reader.read_line(&mut result);
    match result.replace("\n", "").parse::<T>() {
        Ok(x) => {
            if x >= min && x <= max {
                Ok(x)
            } else {
                read_parse(reader.into_inner(), prompt, min, max)
            }
        }
        _ => read_parse(reader.into_inner(), prompt, min, max),
    }
}

fn my_pick_from_list_internal<T: AsRef<str>>(items: &[T], prompt: &str) -> io::Result<usize> {
    let mut tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
    let pad_len = ((items.len() as f32).log10().floor() + 1.0) as usize;
    for (i, item) in items.iter().enumerate() {
        tty.write_all(
            format!(
                "{1:0$}. {2}\n",
                pad_len,
                i + 1,
                item.as_ref().replace("\n", "")
            )
            .as_bytes(),
        )?
    }
    let idx = read_parse::<usize>(&mut tty, prompt, 1, items.len())? - 1;
    Ok(idx)
}

fn print_if_error(result: Result<(), failure::Error>) {
    match result {
        Err(e) => println!("{}", e),
        Ok(_) => {}
    };
}

fn do_command(handler: &mut s3handler::Handler, s3_type: &String, command: &mut String) {
    debug!("===== do command: {} =====", command);
    if command.starts_with("la") {
        match handler.la() {
            Err(e) => println!("{}", e),
            Ok(v) => {
                for o in v {
                    debug!("{:?}", o);
                    println!("{}", String::from(o));
                }
            }
        };
    } else if command.starts_with("ls") {
        match handler.ls(command.split_whitespace().nth(1)) {
            Err(e) => println!("{}", e),
            Ok(v) => {
                for o in v {
                    debug!("{:?}", o);
                    println!("{}", String::from(o));
                }
            }
        };
    } else if command.starts_with("ll") {
        let r = match command.split_whitespace().nth(1) {
            Some(b) => handler.ls(Some(b)),
            None => handler.la(),
        };
        match r {
            Err(e) => println!("{}", e),
            Ok(v) => {
                println!("STORAGE CLASS\tMODIFIED TIME\t\t\tETAG\t\t\t\t\tKEY",);
                for o in v {
                    debug!("{:?}", o);
                    println!(
                        "{}\t{}\t{}\t{}",
                        o.storage_class.clone().unwrap_or("        ".to_string()),
                        o.mtime
                            .clone()
                            .unwrap_or("                        ".to_string()),
                        o.etag
                            .clone()
                            .unwrap_or("                                 ".to_string()),
                        String::from(o)
                    );
                }
            }
        };
    } else if command.starts_with("put") {
        match handler.put(
            command.split_whitespace().nth(1).unwrap_or(""),
            command.split_whitespace().nth(2).unwrap_or(""),
        ) {
            Err(e) => println!("{}", e),
            Ok(_) => println!("upload completed"),
        };
    } else if command.starts_with("get") {
        match handler.get(
            command.split_whitespace().nth(1).unwrap_or(""),
            command.split_whitespace().nth(2),
        ) {
            Err(e) => println!("{}", e),
            Ok(_) => println!("download completed"),
        };

    // It is OK to let it works as POC with something hard coded, because the bounties is not large.
    // Please take this easy.
    // Transfer to IPFS can be a POC start from put file in tmp, then transfer
    } else if command.starts_with("transfer") {

    // Keep this part comment, and you dont really need a S3 account to download something
    //     match handler.get(
    //         command.split_whitespace().nth(1).unwrap_or(""),
    //         "/tmp",
    //     ) {
    //         Err(e) => println!("{}", e),
    //         Ok(_) => println!("download tmp file completed"),
    //     };
    // End of Keep this part comment

    //
    //     // TODO: transfer file to IPFS


        let client = IpfsClient::default();
        let data = Cursor::new("Hello World!");

        let req = client
            .add(data)
            .map(|res| {
                println!("{}", res.hash);
            })
        .map_err(|e| eprintln!("{}", e));

        hyper::rt::run(req);


    //     // TODO: print the Qm hex or 0x hex for the user
    //
    } else if command.starts_with("cat") {
        print_if_error(handler.cat(command.split_whitespace().nth(1).unwrap_or("")));
    } else if command.starts_with("del") || command.starts_with("rm") {
        let mut iter = command.split_whitespace();
        let target = iter.nth(1).unwrap_or("");
        let mut headers = Vec::new();
        loop {
            match iter.next() {
                Some(header_pair) => match header_pair.find(':') {
                    Some(_) => headers.push((
                        header_pair.split(':').nth(0).unwrap(),
                        header_pair.split(':').nth(1).unwrap(),
                    )),
                    None => headers.push((&header_pair, "")),
                },
                None => {
                    break;
                }
            };
        }
        match handler.del_with_flag(target, &mut headers) {
            Err(e) => println!("{}", e),
            Ok(_) => println!("deletion completed"),
        }
    } else if command.starts_with("tag") {
        let mut iter = command.split_whitespace();
        let action = iter.nth(1).unwrap_or("");
        let target = iter.nth(0).unwrap_or("");
        let mut tags = Vec::new();
        loop {
            match iter.next() {
                Some(kv_pair) => match kv_pair.find('=') {
                    Some(_) => tags.push((
                        kv_pair.split('=').nth(0).unwrap(),
                        kv_pair.split('=').nth(1).unwrap(),
                    )),
                    None => tags.push((&kv_pair, "")),
                },
                None => {
                    break;
                }
            };
        }
        match action {
            "add" | "put" => match handler.add_tag(target, &tags) {
                Err(e) => println!("{}", e),
                Ok(_) => println!("tag completed"),
            },
            "del" | "rm" => match handler.del_tag(target) {
                Err(e) => println!("{}", e),
                Ok(_) => println!("tag removed"),
            },
            "ls" | "list" => match handler.list_tag(target) {
                Err(e) => println!("{}", e),
                Ok(_) => {}
            },
            _ => println!("only support these tag actions: ls, add, put, del, rm"),
        }
    } else if command.starts_with("usage") {
        let mut iter = command.split_whitespace();
        let target = iter.nth(1).unwrap_or("");
        let mut options = Vec::new();
        loop {
            match iter.next() {
                Some(kv_pair) => match kv_pair.find('=') {
                    Some(_) => options.push((
                        kv_pair.split('=').nth(0).unwrap(),
                        kv_pair.split('=').nth(1).unwrap(),
                    )),
                    None => options.push((&kv_pair, "")),
                },
                None => {
                    break;
                }
            };
        }
        match handler.usage(target, &options) {
            Err(e) => println!("{}", e),
            Ok(_) => {}
        }
    } else if command.starts_with("mb") {
        print_if_error(handler.mb(command.split_whitespace().nth(1).unwrap_or("")));
    } else if command.starts_with("rb") {
        print_if_error(handler.rb(command.split_whitespace().nth(1).unwrap_or("")));
    } else if command.starts_with("/") {
        match handler.url_command(&command) {
            Err(e) => println!("{}", e),
            Ok(_) => {}
        };
    } else if command.starts_with("info") {
        let target = command.split_whitespace().nth(1).unwrap_or("");
        let caps;
        let bucket = if target.starts_with("s3://") || target.starts_with("S3://") {
            let re = Regex::new(S3_FORMAT).unwrap();
            caps = re
                .captures(command.split_whitespace().nth(1).unwrap_or(""))
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
            }
            "aws" | _ => {}
        }
    } else if command.starts_with("s3_type") {
        handler.change_s3_type(&command);
    } else if command.starts_with("auth_type") {
        handler.change_auth_type(&command);
    } else if command.starts_with("format") {
        handler.change_format_type(&command);
    } else if command.starts_with("url_style") {
        handler.change_url_style(&command);
    } else if command.starts_with("log") {
        change_log_type(&command);
    } else if command.starts_with("exit") || command.starts_with("quit") {
        println!("Thanks for using, cya~");
    } else if command.starts_with("help") {
        println!(
            r#"
USAGE:

{0}
    list all objects

{1}
    list all buckets

{1} s3://{2}
    list all objects of the bucket

{1} s3://{2}/{40}
    list objects with prefix in the bucket

{39}
    list all object detail

{39} s3://{2}
    list all objects detail of the bucket

{39} s3://{2}/{40}
    list detail of the objects with prefix in the bucket

{3} s3://{2}
    create bucket

{4} s3://{2}
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

{10} s3://{2}/{7} [delete-marker:true]
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
            "la".bold(),
            "ls".bold(),
            "<bucket>".cyan(),
            "mb".bold(),
            "rm".bold(),
            "put".bold(),
            "<file>".cyan(),
            "<object>".cyan(),
            "get".bold(),
            "cat".bold(),
            "del".bold(),
            "<uri>".cyan(),
            "<query string>".cyan(),
            "help".bold(),
            "log".bold(),
            "trace".blue(),
            "debug".blue(),
            "info".blue(),
            "error".blue(),
            "s3_type".bold(),
            "aws".blue(),
            "ceph".blue(),
            "auth_type".bold(),
            "aws2".blue(),
            "aws4".blue(),
            "format".bold(),
            "xml".blue(),
            "json".blue(),
            "exit".bold(),
            "tag".bold(),
            "<key>".cyan(),
            "<value>".cyan(),
            "trace".blue(),
            "add".bold(),
            "logout".bold(),
            "Ctrl + d".bold(),
            "list".bold(),
            "usage".bold(),
            "info".bold(),
            "ll".bold(),
            "<prefix>".cyan(), //40
        );
    } else {
        println!(
            "command {} not found, help for usage or exit for quit",
            command
        );
    }
}

fn main() {
    log::set_logger(&MY_LOGGER).unwrap();
    log::set_max_level(LevelFilter::Error);

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("S3RS CONFIGURE")
                .help("set the name of config file under ~/.config/s3rs, or a config file with fullpath")
        )
        .arg(
            Arg::from_usage("<s3rs cmd>...").requires("config").required(false)
        )
        .get_matches();

    let mut config_contents = String::new();
    let interactive: bool;
    let s3rs_config_foler = home_dir().unwrap().join(S3RS_CONFIG_FOLDER); // used > v0.2.3
    match matches.value_of("config") {
        Some(path) => {
            let mut config_path = path.to_string();
            if s3rs_config_foler.exists() {
                for entry in read_dir(s3rs_config_foler).unwrap() {
                    let p = entry.unwrap().path();
                    if path == p.file_stem().unwrap().to_str().unwrap_or("") {
                        config_path = format!("{}", p.to_str().unwrap_or(""));
                    }
                }
            };

            let mut f = File::open(&config_path).expect("cannot open file");
            f.read_to_string(&mut config_contents)
                .expect("cannot read file");
            interactive = false;
        }
        None => {
            let legacy_s3rs_config = home_dir().unwrap().join(".s3rs.toml"); // used < v0.2.2

            if s3rs_config_foler.exists() {
                for entry in read_dir(s3rs_config_foler).unwrap() {
                    let path = entry.unwrap().path();
                    if !path.is_dir() {
                        let mut f = File::open(path).expect("cannot open file");
                        f.read_to_string(&mut config_contents)
                            .expect("cannot read file");
                    }
                }
            } else if legacy_s3rs_config.exists() {
                println!("{}", "legacy s3rs config file detected, you may split it into different config files, and put them under ~/.config/s3rs".bold());
                let mut f =
                    File::open(legacy_s3rs_config).expect("Can not open legacy 3rs config file");
                f.read_to_string(&mut config_contents)
                    .expect("legacy s3rs config is not readable");
            } else {
                create_dir(s3rs_config_foler.clone()).expect("create config folder fail");
                let mut f = File::create(s3rs_config_foler.join("aws-example.toml"))
                    .expect("Can not write s3rs config example file");
                let _ = f.write_all(include_str!("../config_examples/aws-example.toml").as_bytes());
                let mut f = File::create(s3rs_config_foler.join("ceph-example.toml"))
                    .expect("Can not write s3rs config example file");
                let _ =
                    f.write_all(include_str!("../config_examples/ceph-example.toml").as_bytes());
                println!(
                    "Example files is created in {}, multiple toml files can put under this folder",
                    "~/.config/s3rs".bold()
                );
                return;
            }

            if config_contents == "" {
                println!(
                    "{}",
                    "Lack of config files please put in ~/.config/s3rs".bold()
                );
                return;
            }
            interactive = true;
        }
    };

    let config: Config = toml::from_str(config_contents.as_str()).unwrap();
    let config_option: Vec<String> = config.gen_selecitons();

    let mut chosen_int = if config_option.len() == 1 {
        0usize
    } else {
        my_pick_from_list_internal(&config_option, "Selection: ").unwrap()
    };

    let config_list = config.credential.unwrap();
    let mut handler = s3handler::Handler::from(&config_list[chosen_int]);
    let mut login_user = config_list[chosen_int]
        .user
        .clone()
        .unwrap_or("unknown".to_string());
    let mut s3_type = config_list[chosen_int]
        .s3_type
        .clone()
        .unwrap_or("aws".to_string());

    if matches.value_of("config").is_none() {
        println!(
            "enter command, type {} for usage or type {} for quit",
            "help".bold(),
            "exit".bold()
        );
    };

    // let mut raw_input;
    let mut command = match values_t!(matches, "s3rs cmd", String) {
        Ok(cmds) => cmds.join(" "),
        Err(_) => String::new(),
    };

    while command != "exit" && command != "quit" {
        if command.starts_with("logout") {
            println!("");
            chosen_int = my_pick_from_list_internal(&config_option, "Selection: ").unwrap();
            handler = s3handler::Handler::from(&config_list[chosen_int]);
            login_user = config_list[chosen_int]
                .user
                .clone()
                .unwrap_or(" ".to_string());
            s3_type = config_list[chosen_int]
                .s3_type
                .clone()
                .unwrap_or("aws".to_string());
        } else {
            do_command(&mut handler, &s3_type, &mut command);
        }

        if !interactive {
            break;
        }

        command = match OpenOptions::new().read(true).write(true).open("/dev/tty") {
            Ok(mut tty) => {
                tty.flush().expect("Could not open tty");
                let _ = tty.write_all(
                    format!("{} {} {} ", "s3rs".green(), login_user.cyan(), ">".green()).as_bytes(),
                );
                let reader = BufReader::new(&tty);
                let mut command_iter = reader.lines().map(|l| l.unwrap());
                command_iter.next().unwrap_or("logout".to_string())
            }
            Err(e) => {
                println!("{:?}", e);
                "quit".to_string()
            }
        };

        println!("");
        stdout().flush().expect("Could not flush stdout");
    }
}
