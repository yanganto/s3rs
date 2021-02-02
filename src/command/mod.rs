use regex::Regex;
use humansize::{FileSize, file_size_opts};

use crate::logger::change_log_type;
use colored::{self, *};

pub mod secret;

static S3_FORMAT: &'static str =
    r#"[sS]3://(?P<bucket>[A-Za-z0-9\-\._]+)(?P<object>[A-Za-z0-9\-\._/]*)"#;

fn print_if_error(result: Result<(), failure::Error>) {
    match result {
        Err(e) => println!("{}", e),
        Ok(_) => {}
    };
}

pub fn common_usage() -> String {
    let usage = format!(
        r#"
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
    usage
}

pub fn do_command(handler: &mut s3handler::Handler, s3_type: &String, command: &mut String) {
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
                println!("STORAGE CLASS\tMODIFIED TIME\t\t\tETAG\t\t\t\t\tSIZE\tKEY",);
                for o in v {
                    debug!("{:?}", o);
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        o.storage_class.clone().unwrap_or("        ".to_string()),
                        o.mtime
                            .clone()
                            .unwrap_or("                        ".to_string()),
                        o.etag
                            .clone()
                            .unwrap_or("                                 ".to_string()),
                        o.size.map(|s| s.file_size(file_size_opts::CONVENTIONAL).unwrap()).unwrap_or_else(|| "".to_string()),
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
S3RS COMMAND:"#
        );
        println!("{}", common_usage());
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
