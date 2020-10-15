#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use std::fs::{create_dir, read_dir, File, OpenOptions};
use std::io;
use std::io::stdout;
use std::io::{BufRead, BufReader, Read, Write};
use std::str;
use std::str::FromStr;

use clap::{App, Arg, ArgMatches};
use colored::{self, *};
use dirs::home_dir;
use log::LevelFilter;
use regex::Regex;

use command::{print_usage, secret};
use config::Config;
use logger::{change_log_type, Logger};

mod command;
mod config;
mod logger;

static MY_LOGGER: Logger = Logger;
static S3_FORMAT: &'static str =
    r#"[sS]3://(?P<bucket>[A-Za-z0-9\-\._]+)(?P<object>[A-Za-z0-9\-\._/]*)"#;
static S3RS_CONFIG_FOLDER: &'static str = ".config/s3rs";

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
    } else if command.starts_with("cat") {
        if let Ok(o) = handler.cat(command.split_whitespace().nth(1).unwrap_or("")) {
            println!("{}", o.1.unwrap_or("".to_string()));
        } else {
            error!("can not cat the object");
        }
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
USAGE:"#
        );
        print_usage();
        secret::print_usage();
        println!(
            "If you have any issue, please submit to here https://github.com/yanganto/s3rs/issues"
        );
    } else {
        println!(
            "command {} not found, help for usage or exit for quit",
            command
        );
    }
}
fn cli() -> ArgMatches<'static> {
    App::new(env!("CARGO_PKG_NAME"))
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
            Arg::with_name("secret")
                .short("s")
                .long("secret")
                .value_name("Runtime Secret")
                .help("Set the run time secret to encrypt/decrept your s3config file")
		)
        .arg(
            Arg::from_usage("<s3rs cmd>...").requires("config").required(false)
        )
        .get_matches()
}

fn main() {
    log::set_logger(&MY_LOGGER).unwrap();
    log::set_max_level(LevelFilter::Warn);

    let mut run_time_secret: Vec<u8> = Vec::new();
    let matches = cli();
    if let Some(s) = matches.value_of("secret") {
        secret::change_secret(&mut run_time_secret, s.to_string(), false);
    }

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

    let mut config: Config = toml::from_str(config_contents.as_str()).unwrap();

    let mut reload_config = true;

    while reload_config {
        reload_config = false;
        config.decrypt(&run_time_secret);
        let config_option: Vec<String> = config.gen_selections();
        let chosen_int = if config_option.len() == 1 {
            0usize
        } else {
            my_pick_from_list_internal(&config_option, "Selection: ").unwrap()
        };
        let config_list = config.credential.clone().unwrap().clone();
        let mut handler = s3handler::Handler::from(&config_list[chosen_int]);
        let login_user = config_list[chosen_int]
            .user
            .clone()
            .unwrap_or("unknown".to_string());
        let s3_type = config_list[chosen_int]
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
                reload_config = true;
                break;
            } else if command.starts_with("secret") {
                let mut command = command.strip_prefix("secret").unwrap().trim().to_string();
                secret::do_command(&mut run_time_secret, &mut command, &config_list, chosen_int)
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
                        format!("{} {} {} ", "s3rs".green(), login_user.cyan(), ">".green())
                            .as_bytes(),
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
}
