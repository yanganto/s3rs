#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

use std::fs::{create_dir, read_dir, File, OpenOptions};
use std::io;
use std::io::stdout;
use std::io::{BufRead, BufReader, Read, Write};
use std::str;
use std::str::FromStr;

use colored::{self, *};
use dirs::home_dir;
use log::LevelFilter;
use structopt::StructOpt;

use command::{do_command, secret, Cli, S3rsCmd};
use config::Config;
use logger::Logger;

mod command;
mod config;
mod logger;

static MY_LOGGER: Logger = Logger;
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

fn main() -> io::Result<()> {
    log::set_logger(&MY_LOGGER).unwrap();
    log::set_max_level(LevelFilter::Warn);

    let mut run_time_secret: Vec<u8> = Vec::new();
    let mut matches = Cli::from_args();
    if let Some(s) = matches.secret {
        secret::change_secret(&mut run_time_secret, s.to_string(), false);
    }

    let mut config_contents = String::new();
    let mut interactive: bool;
    let s3rs_config_foler = home_dir().unwrap().join(S3RS_CONFIG_FOLDER); // used > v0.2.3
    match matches.config {
        Some(ref path) => {
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
            let args: Vec<String> = std::env::args().collect();
            matches.s3rs_cmd = S3rsCmd::from_iter_safe(args[1..].iter()).ok();
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
                return Ok(());
            }

            if config_contents == "" {
                println!(
                    "{}",
                    "Lack of config files please put in ~/.config/s3rs".bold()
                );
                return Ok(());
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

        if matches.config.is_none() {
            println!(
                "enter command, type {} for usage or type {} for quit",
                "help".bold(),
                "exit".bold()
            );
        };

        let mut command = String::new();
        while matches.s3rs_cmd != Some(S3rsCmd::Quit) {
            stdout().flush().expect("Could not flush stdout");

            if command.starts_with("logout") {
                reload_config = true;
                break;
            } else if command.starts_with("secret") {
                let mut command = command.strip_prefix("secret").unwrap().trim().to_string();
                secret::do_command(&mut run_time_secret, &mut command, &config_list, chosen_int)
            } else if command.starts_with("exit") || command.starts_with("quit") {
                interactive = false;
            } else {
                do_command(&mut handler, &s3_type, matches.s3rs_cmd.take());
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

            matches.s3rs_cmd = if command.starts_with('/') {
                Some(S3rsCmd::Query {
                    url: command.clone(),
                })
            } else {
                let mut new_s3_cmd = vec![""];
                new_s3_cmd.append(&mut command.split_whitespace().collect());
                S3rsCmd::from_iter_safe(new_s3_cmd).ok()
            };
        }
    }
    Ok(())
}
