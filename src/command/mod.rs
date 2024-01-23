use std::error::Error;

use humansize::{make_format_i, DECIMAL};
use regex::Regex;
#[cfg(feature = "async")]
use std::path::Path;
#[cfg(feature = "async")]
use tokio::runtime::Runtime;

use crate::logger::{change_log_type, LogType};
use clap::{Parser, ValueEnum};
use colored::{self, *};
#[cfg(feature = "async")]
use s3handler::{none_blocking::primitives::S3Pool, S3Object};

pub mod secret;

static S3_FORMAT: &str = r#"[sS]3://(?P<bucket>[A-Za-z0-9\-\._]+)(?P<object>[A-Za-z0-9\-\._/]*)"#;

#[derive(Parser, Debug)]
#[command(name = "s3rs")]
pub struct Cli {
    /// Set the name of config file under ~/.config/s3rs, or a config file with fullpath
    #[arg(short, long)]
    pub(crate) config: Option<String>,

    /// Set the run time secret to encrypt/decrept your s3config file
    #[arg(short, long)]
    pub(crate) secret: Option<String>,

    #[command(subcommand)]
    pub s3rs_cmd: Option<S3rsCmd>,
}

#[derive(Parser, PartialEq, Debug)]
pub enum S3rsCmd {
    #[command(name = "la", about = "list all buckets")]
    ListAll,

    #[command(
        name = "ls",
        about = r#"list all buckets, or
list all objects of the bucket, or
    ls s3://<bucket>
list objects with prefix in the bucket
    ls s3://<bucket>/<prefix>"#
    )]
    List { uri: Option<String> },

    #[command(
        name = "ll",
        about = r#"list all object detail, or
list all objects detail of the bucket, or
    ll s3://<bucket>
list detail of the objects with prefix in the bucket
    ls s3://<bucket>/<prefix>"#
    )]
    Detail { uri: Option<String> },

    #[command(
        name = "mb",
        about = r#"create bucket
    mb s3://<bucket>"#
    )]
    CreateBucket { bucket: String },

    #[command(
        name = "rb",
        about = r#"delete bucket
    rb s3://<bucket>"#
    )]
    DeleteBucket { bucket: String },

    #[command(about = r#"upload the file with specify object name
    put <file> s3://<bucket>/<object>
upload the file as the same file name
    put <file> s3://<bucket>
upload a small test text file with specify object name
    put test s3://<bucket>/<object>"#)]
    Put { file: String, uri: String },

    #[command(about = r#"download the object
    get s3://<bucket>/<object> <file>
download the object to current folder
    get s3://<bucket>/<object>
upload a small test text file with specify object name
    put test s3://<bucket>/<object>"#)]
    Get { uri: String, file: Option<String> },

    #[command(about = r#"display the object content
    cat s3://<bucket>/<object>"#)]
    Cat { uri: String },

    #[command(about = r#"delete the object with/out delete marker
    del s3://<bucket>/<object> [delete-marker:true]"#)]
    Del { uri: String, marker: Option<String> },

    #[command(about = r#"delete the object with/out delete marker
    rm s3://<bucket>/<object> [delete-marker:true]"#)]
    Rm { uri: String, marker: Option<String> },

    #[command(about = r#"tag operations
list tags of the object
    tag ls/list s3://<bucket>/<object>
add tags to the object
    tag add/put s3://<bucket>/<object>  <key>=<value> ...
remove tags from the object
    tag del/rm s3://<bucket>/<object>"#)]
    Tag {
        #[arg(value_enum)]
        action: TagAction,
        uri: String,
        tags: Vec<String>,
    },

    #[command(
        name = "/",
        about = r#"get uri command
    /<uri>?<query string>"#
    )]
    Query { url: String },

    #[command(about = r#"change the log level
trace for every thing including request auth detail
debug for request header, status code, raw body
info for request http response
error is default
    log trace/trace/debug/info/error"#)]
    Log {
        #[arg(value_enum)]
        log_type: LogType,
    },

    #[command(
        name = "s3_type",
        about = r#"change the auth type and format for different S3 service
    s3_type aws/ceph"#
    )]
    S3Type {
        #[arg(value_enum)]
        s3_type: S3Type,
    },

    #[command(
        name = "auth_type",
        about = r#"change the auth type
    auth_type aws2/aws4"#
    )]
    AuthType {
        #[arg(value_enum)]
        auth_type: AuthType,
    },

    #[command(about = r#"change the request format
    format xml/json"#)]
    Format {
        #[arg(value_enum)]
        format_type: FormatType,
    },

    #[command(about = r#"change the request url style
    url-style path/host"#)]
    UrlStyle {
        #[arg(value_enum)]
        url_style: UrlStyle,
    },

    #[command(name = "logout/Ctrl + d", about = "logout and reselect account")]
    Logout,

    #[command(about = r#"show the usage of the bucket (ceph admin only)
    usage s3://<bucket>"#)]
    Usage {
        bucket: String,
        options: Option<String>,
    },

    #[command(
        about = r#"show following the bucket information, acl(ceph, aws), location(ceph, aws), versioning(ceph, aws), uploads(ceph), version(ceph)
    info s3://<bucket>"#
    )]
    Info { bucket: String },

    #[command(name = "quit/exit", about = "quit the programe")]
    Quit,
    // #[command(name = "help", about = "show s3 command usage")]
    // Help,
}

#[derive(ValueEnum, PartialEq, Debug, Clone)]
pub enum TagAction {
    List,
    Ls,
    Add,
    Put,
    Del,
    Rm,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(ValueEnum, PartialEq, Debug, Clone)]
pub enum S3Type {
    AWS,
    CEPH,
}

impl From<S3Type> for &'static str {
    fn from(val: S3Type) -> Self {
        match val {
            S3Type::AWS => "aws",
            S3Type::CEPH => "ceph",
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(ValueEnum, PartialEq, Debug, Clone)]
pub enum AuthType {
    AWS2,
    AWS4,
}

impl From<AuthType> for &'static str {
    fn from(val: AuthType) -> Self {
        match val {
            AuthType::AWS2 => "aws2",
            AuthType::AWS4 => "aws4",
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(ValueEnum, PartialEq, Debug, Clone)]
pub enum FormatType {
    XML,
    JSON,
}

impl From<FormatType> for &'static str {
    fn from(val: FormatType) -> Self {
        match val {
            FormatType::XML => "xml",
            FormatType::JSON => "json",
        }
    }
}

#[derive(ValueEnum, PartialEq, Debug, Clone)]
pub enum UrlStyle {
    Path,
    Host,
}

impl From<UrlStyle> for &'static str {
    fn from(val: UrlStyle) -> Self {
        match val {
            UrlStyle::Path => "path",
            UrlStyle::Host => "host",
        }
    }
}

fn print_if_error(result: Result<(), Box<dyn Error>>) {
    if let Err(e) = result {
        println!("{}", e)
    }
}

pub fn do_command(handler: &mut s3handler::Handler, s3_type: &str, command: Option<S3rsCmd>) {
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
                                .map(&size_formatter)
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
            for header_pair in marker.as_deref().unwrap_or("").split_whitespace() {
                match header_pair.find(':') {
                    Some(_) => headers.push((
                        header_pair.split(':').next().unwrap_or_default(),
                        header_pair.split(':').nth(1).unwrap_or_default(),
                    )),
                    None => headers.push((&header_pair, "")),
                }
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
        })
        | Some(S3rsCmd::Tag {
            action: TagAction::Ls,
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
        })
        | Some(S3rsCmd::Tag {
            action: TagAction::Put,
            uri,
            tags,
        }) => {
            let mut tags_vec = Vec::new();
            for kv_pair in tags.iter() {
                match kv_pair.find('=') {
                    Some(_) => tags_vec.push((
                        kv_pair.split('=').next().unwrap_or_default(),
                        kv_pair.split('=').nth(1).unwrap_or_default(),
                    )),
                    None => tags_vec.push((&kv_pair, "")),
                }
            }
            if let Err(e) = handler.add_tag(&uri, &tags_vec) {
                println!("{}", e);
            }
        }
        Some(S3rsCmd::Tag {
            action: TagAction::Del,
            uri,
            ..
        })
        | Some(S3rsCmd::Tag {
            action: TagAction::Rm,
            uri,
            ..
        }) => {
            if let Err(e) = handler.del_tag(&uri) {
                println!("{}", e);
            }
        }
        Some(S3rsCmd::Usage { bucket, options }) => {
            let mut options_vec = Vec::new();
            for kv_pair in options.as_deref().unwrap_or("").split_whitespace() {
                match kv_pair.find('=') {
                    Some(_) => options_vec.push((
                        kv_pair.split('=').next().unwrap_or_default(),
                        kv_pair.split('=').nth(1).unwrap_or_default(),
                    )),
                    None => options_vec.push((&kv_pair, "")),
                }
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
            if s3_type == "ceph" {
                println!("\n{}", "version:".yellow().bold());
                let _ = handler.url_command(format!("/{}?version", bucket).as_str());
                println!("\n{}", "uploads:".yellow().bold());
                let _ = handler.url_command(format!("/{}?uploads", bucket).as_str());
            }
        }
        Some(S3rsCmd::S3Type { s3_type }) => {
            handler.change_s3_type(s3_type.into());
        }
        Some(S3rsCmd::AuthType { auth_type }) => {
            handler.change_auth_type(auth_type.into());
        }
        Some(S3rsCmd::Format { format_type }) => {
            handler.change_format_type(format_type.into());
        }
        Some(S3rsCmd::UrlStyle { url_style }) => {
            handler.change_url_style(url_style.into());
        }
        Some(S3rsCmd::Log { log_type }) => {
            change_log_type(&log_type);
        }
        None | Some(S3rsCmd::Logout) | Some(S3rsCmd::Quit) => (), // handle in main loop
    }
}
