use std::error::Error;

use humansize::{make_format_i, DECIMAL};
use regex::Regex;
#[cfg(feature = "async")]
use std::path::Path;
#[cfg(feature = "async")]
use tokio::runtime::Runtime;

use crate::logger::{change_log_type, LogType};
use colored::{self, *};
#[cfg(feature = "async")]
use s3handler::{none_blocking::primitives::S3Pool, S3Object};
use structopt::StructOpt;

pub mod secret;

static S3_FORMAT: &'static str =
    r#"[sS]3://(?P<bucket>[A-Za-z0-9\-\._]+)(?P<object>[A-Za-z0-9\-\._/]*)"#;

#[derive(StructOpt, Debug)]
#[structopt(name = "s3rs")]
pub struct Cli {
    /// Set the name of config file under ~/.config/s3rs, or a config file with fullpath
    #[structopt(short = "c", long)]
    pub(crate) config: Option<String>,

    /// Set the run time secret to encrypt/decrept your s3config file
    #[structopt(short = "s", long)]
    pub(crate) secret: Option<String>,

    #[structopt(subcommand)]
    pub s3rs_cmd: Option<S3rsCmd>,
}

#[derive(StructOpt, PartialEq, Debug)]
#[structopt()]
pub enum S3rsCmd {
    #[structopt(name = "la", about = "list all buckets")]
    ListAll,

    #[structopt(
        name = "ls",
        about = r#"list all buckets, or
list all objects of the bucket, or
    ls s3://<bucket>
list objects with prefix in the bucket
    ls s3://<bucket>/<prefix>"#
    )]
    List { uri: Option<String> },

    #[structopt(
        name = "ll",
        about = r#"list all object detail, or
list all objects detail of the bucket, or
    ll s3://<bucket>
list detail of the objects with prefix in the bucket
    ls s3://<bucket>/<prefix>"#
    )]
    Detail { uri: Option<String> },

    #[structopt(
        name = "mb",
        about = r#"create bucket
    mb s3://<bucket>"#
    )]
    CreateBucket { bucket: String },

    #[structopt(
        name = "rb",
        about = r#"delete bucket
    rb s3://<bucket>"#
    )]
    DeleteBucket { bucket: String },

    #[structopt(about = r#"upload the file with specify object name
    put <file> s3://<bucket>/<object>
upload the file as the same file name
    put <file> s3://<bucket>
upload a small test text file with specify object name
    put test s3://<bucket>/<object>"#)]
    Put { file: String, uri: String },

    #[structopt(about = r#"download the object
    get s3://<bucket>/<object> <file>
download the object to current folder
    get s3://<bucket>/<object>
upload a small test text file with specify object name
    put test s3://<bucket>/<object>"#)]
    Get { uri: String, file: Option<String> },

    #[structopt(about = r#"display the object content
    cat s3://<bucket>/<object>"#)]
    Cat { uri: String },

    #[structopt(about = r#"delete the object with/out delete marker
    del s3://<bucket>/<object> [delete-marker:true]"#)]
    Del { uri: String, marker: Option<String> },

    #[structopt(about = r#"delete the object with/out delete marker
    rm s3://<bucket>/<object> [delete-marker:true]"#)]
    Rm { uri: String, marker: Option<String> },

    #[structopt(about = r#"tag operations
list tags of the object
    tag ls/list s3://<bucket>/<object>
add tags to the object
    tag add/put s3://<bucket>/<object>  <key>=<value> ...
remove tags from the object
    tag del/rm s3://<bucket>/<object>"#)]
    Tag {
        action: TagAction,
        uri: String,
        tags: Option<String>,
    },

    #[structopt(
        name = "/",
        about = r#"get uri command
    /<uri>?<query string>"#
    )]
    Query { url: String },

    #[structopt(about = r#"change the log level
trace for every thing including request auth detail
debug for request header, status code, raw body
info for request http response
error is default
    log trace/trace/debug/info/error"#)]
    Log(LogType),

    #[structopt(
        name = "s3_type",
        about = r#"change the auth type and format for different S3 service
    s3_type aws/ceph"#
    )]
    S3Type(S3Type),

    #[structopt(
        name = "auth_type",
        about = r#"change the auth type
    auth_type aws2/aws4"#
    )]
    AuthType(AuthType),

    #[structopt(about = r#"change the request format
    format xml/json"#)]
    Format(AuthType),

    #[structopt(about = r#"change the request url style
    url-style path/host"#)]
    UrlStyle(AuthType),

    #[structopt(name = "logout/Ctrl + d", about = "logout and reselect account")]
    Logout,

    #[structopt(about = r#"show the usage of the bucket (ceph admin only)
    usage s3://<bucket>"#)]
    Usage {
        bucket: String,
        options: Option<String>,
    },

    #[structopt(
        about = r#"show following the bucket information, acl(ceph, aws), location(ceph, aws), versioning(ceph, aws), uploads(ceph), version(ceph)
    info s3://<bucket>"#
    )]
    Info { bucket: String },

    #[structopt(name = "quit/exit", about = "quit the programe")]
    Quit,
}

#[derive(StructOpt, PartialEq, Debug)]
pub enum TagAction {
    List,
    Add,
    Delete,
}

impl std::str::FromStr for TagAction {
    type Err = Box<dyn std::error::Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "list" | "ls" => Ok(TagAction::List),
            "add" | "put" => Ok(TagAction::Add),
            "del" | "rm" => Ok(TagAction::Delete),
            _ => {
                println!("only support these tag actions: list, ls, add, put, del, rm");
                Err("Unknown tag action".into())
            }
        }
    }
}

impl std::fmt::Display for TagAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagAction::List => {
                write!(f, "list tag")
            }
            TagAction::Add => {
                write!(f, "add tag")
            }
            TagAction::Delete => {
                write!(f, "delete tag")
            }
        }
    }
}

#[derive(StructOpt, PartialEq, Debug)]
pub enum S3Type {
    AWS,
    CEPH,
}

impl Into<&'static str> for S3Type {
    fn into(self) -> &'static str {
        match self {
            Self::AWS => "aws",
            Self::CEPH => "ceph",
        }
    }
}

#[derive(StructOpt, PartialEq, Debug)]
pub enum AuthType {
    AWS2,
    AWS4,
}

impl Into<&'static str> for AuthType {
    fn into(self) -> &'static str {
        match self {
            Self::AWS2 => "aws2",
            Self::AWS4 => "aws4",
        }
    }
}

#[derive(StructOpt, PartialEq, Debug)]
pub enum FormatType {
    XML,
    JSON,
}

impl Into<&'static str> for FormatType {
    fn into(self) -> &'static str {
        match self {
            Self::XML => "xml",
            Self::JSON => "json",
        }
    }
}

#[derive(StructOpt, PartialEq, Debug)]
pub enum UrlStyle {
    Path,
    Host,
}

impl Into<&'static str> for UrlStyle {
    fn into(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Host => "host",
        }
    }
}

// XXX show in shell
// help
//     show this usage

fn print_if_error(result: Result<(), Box<dyn Error>>) {
    match result {
        Err(e) => println!("{}", e),
        Ok(_) => {}
    };
}

pub fn do_command(handler: &mut s3handler::Handler, s3_type: &String, command: Option<S3rsCmd>) {
    debug!("===== do command: {:?} =====", command);
    match command {
        Some(S3rsCmd::ListAll) => match handler.la() {
            Err(e) => println!("{}", e),
            Ok(v) => {
                for o in v {
                    debug!("{:?}", o);
                    println!("{}", String::from(o));
                }
            }
        },
        Some(S3rsCmd::List { uri }) => match handler.ls(uri.as_deref()) {
            Err(e) => println!("{}", e),
            Ok(v) => {
                for o in v {
                    debug!("{:?}", o);
                    println!("{}", String::from(o));
                }
            }
        },
        Some(S3rsCmd::Detail { uri }) => {
            let r = match uri {
                Some(b) => handler.ls(Some(&b)),
                None => handler.la(),
            };
            let size_formatter = make_format_i(DECIMAL);
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
                            o.size
                                .map(|s| size_formatter(s))
                                .unwrap_or_else(|| "".to_string()),
                            String::from(o)
                        );
                    }
                }
            };
        }
        Some(S3rsCmd::Put { uri, file }) => {
            #[cfg(feature = "async")]
            {
                let rt = Runtime::new().unwrap();
                let file_path = &file;
                let mut s3_object: S3Object = (&*uri).into();
                if s3_object.key.is_none() {
                    s3_object.key = Some(
                        Path::new(file_path)
                            .file_name()
                            .map(|s| format!("/{}", s.to_string_lossy()))
                            .unwrap_or_else(|| "/".to_string()),
                    )
                }
                let s3_pool = S3Pool::from(&*handler);
                rt.block_on(async {
                    match s3_pool.resource(s3_object).upload_file(file_path).await {
                        Err(e) => println!("{}", e),
                        Ok(_) => println!("upload completed"),
                    };
                });
            }

            #[cfg(not(feature = "async"))]
            match handler.put(&file, &uri) {
                Err(e) => println!("{}", e),
                Ok(_) => println!("upload completed"),
            };
        }
        Some(S3rsCmd::Get { uri, file }) => {
            #[cfg(feature = "async")]
            {
                let s3_pool = S3Pool::from(&*handler);
                let s3_object: S3Object = (&*uri).into();
                if s3_object.key.is_none() {
                    println!("please specify the object you want to download");
                    return;
                }

                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    match s3_pool
                        .resource(s3_object)
                        .download_file(file.as_deref().unwrap_or(""))
                        .await
                    {
                        Err(e) => println!("{}", e),
                        Ok(_) => println!("download completed"),
                    };
                });
            }

            #[cfg(not(feature = "async"))]
            match handler.get(uri.as_dref().file) {
                Err(e) => println!("{}", e),
                Ok(_) => println!("download completed"),
            };
        }
        Some(S3rsCmd::Cat { uri }) => {
            if let Ok(o) = handler.cat(&uri) {
                println!("{}", o.1.unwrap_or("".to_string()));
            } else {
                error!("can not cat the object");
            }
        }
        Some(S3rsCmd::Del { uri, marker }) | Some(S3rsCmd::Rm { uri, marker }) => {
            let target = &uri;
            let mut headers = Vec::new();
            let mut iter = marker.as_deref().unwrap_or("").split_whitespace();
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
        }
        Some(S3rsCmd::Tag {
            action: TagAction::List,
            uri,
            ..
        }) => {
            if let Err(e) = handler.list_tag(&uri) {
                println!("{}", e);
            }
        }
        Some(S3rsCmd::Tag {
            action: TagAction::Add,
            uri,
            tags,
        }) => {
            let mut iter = tags.as_deref().unwrap_or("").split_whitespace();
            let mut tags_vec = Vec::new();
            loop {
                match iter.next() {
                    Some(kv_pair) => match kv_pair.find('=') {
                        Some(_) => tags_vec.push((
                            kv_pair.split('=').nth(0).unwrap(),
                            kv_pair.split('=').nth(1).unwrap(),
                        )),
                        None => tags_vec.push((&kv_pair, "")),
                    },
                    None => {
                        break;
                    }
                };
            }
            if let Err(e) = handler.add_tag(&uri, &tags_vec) {
                println!("{}", e);
            }
        }
        Some(S3rsCmd::Tag {
            action: TagAction::Delete,
            uri,
            ..
        }) => {
            if let Err(e) = handler.del_tag(&uri) {
                println!("{}", e);
            }
        }
        Some(S3rsCmd::Usage { bucket, options }) => {
            let mut iter = options.as_deref().unwrap_or("").split_whitespace();
            let mut options_vec = Vec::new();
            loop {
                match iter.next() {
                    Some(kv_pair) => match kv_pair.find('=') {
                        Some(_) => options_vec.push((
                            kv_pair.split('=').nth(0).unwrap(),
                            kv_pair.split('=').nth(1).unwrap(),
                        )),
                        None => options_vec.push((&kv_pair, "")),
                    },
                    None => {
                        break;
                    }
                };
            }
            if let Err(e) = handler.usage(&bucket, &options_vec) {
                println!("{}", e);
            }
        }
        Some(S3rsCmd::CreateBucket { bucket }) => {
            print_if_error(handler.mb(&bucket));
        }
        Some(S3rsCmd::DeleteBucket { bucket }) => {
            print_if_error(handler.rb(&bucket));
        }
        Some(S3rsCmd::Query { url }) => {
            if let Err(e) = handler.url_command(&url) {
                println!("{}", e);
            }
        }
        Some(S3rsCmd::Info { bucket }) => {
            let caps;
            let bucket = if bucket.starts_with("s3://") || bucket.starts_with("S3://") {
                let re = Regex::new(S3_FORMAT).unwrap();
                caps = re.captures(&bucket).expect("S3 object format error.");
                &caps["bucket"]
            } else {
                &bucket
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
        }
        Some(S3rsCmd::S3Type(t)) => {
            handler.change_s3_type(t.into());
        }
        Some(S3rsCmd::AuthType(t)) => {
            handler.change_auth_type(t.into());
        }
        Some(S3rsCmd::Format(t)) => {
            handler.change_format_type(t.into());
        }
        Some(S3rsCmd::UrlStyle(t)) => {
            handler.change_url_style(t.into());
        }
        Some(S3rsCmd::Log(t)) => {
            change_log_type(&t);
        }
        None | Some(S3rsCmd::Logout) | Some(S3rsCmd::Quit) => (), // handle in main loop
    }
}
