use std;
use std::fs::{File, write};
use std::io::prelude::*;
use std::str::FromStr;
use std::path::Path;

use chrono::prelude::*;
use reqwest::{Response, header, Client};
use serde_json;
use regex::Regex;
use quick_xml::Reader;
use quick_xml::events::Event;



mod aws;

static S3_FORMAT: &'static str = r#"[sS]3://(?P<bucket>[A-Za-z0-9\-\._]+)(?P<object>[A-Za-z0-9\-\._/]*)"#;
static RESPONSE_FORMAT: &'static str = r#""Contents":\["([A-Za-z0-9\-\._]+?)"(.*?)\]"#;


pub enum AuthType{
    AWS4,
    AWS2,
}

pub enum Format{
    JSON,
    XML
}

pub enum UrlStyle{
    PATH,
    HOST
}

pub struct Handler<'a>{
    pub host: &'a str,
    pub access_key: &'a str,
    pub secrete_key: &'a str,
    pub auth_type:AuthType,
    pub format: Format,
    pub url_style: UrlStyle,
    pub region: Option<String>
}

fn print_response(res: &mut Response) -> Vec<u8>{
    let mut body = Vec::new();
    let _ =res.read_to_end(&mut body);
    if res.status().is_success() {
        info!("Status: {}", res.status());
        info!("Headers:\n{:?}", res.headers());
        info!("Body:\n{}\n\n", std::str::from_utf8(&body).expect("Body can not decode as UTF8"));
    } else {
        error!("Status: {}", res.status());
        error!("Headers:\n{:?}", res.headers());
        error!("Body:\n{}\n\n", std::str::from_utf8(&body).expect("Body can not decode as UTF8"));
    }
    body 
}

impl<'a> Handler<'a>  {
    fn aws_v2_request(&self, method: &str, uri: &str, qs: &Vec<(&str, &str)>, payload: &Vec<u8>) -> Result<Vec<u8>, &'static str>{

        let utc: DateTime<Utc> = Utc::now();   
        let mut headers = header::HeaderMap::new();
        let time_str = utc.to_rfc2822();
        headers.insert("date", time_str.clone().parse().unwrap());

        // NOTE: ceph has bug using x-amz-date
        let mut signed_headers = vec![
            ("Date", time_str.as_str())
        ];

        let mut query_strings = vec![];
        match self.format {
            Format::JSON => {query_strings.push(("format", "json"))}
            _ => {}
        }

        query_strings.extend(qs.iter().cloned());

        let mut query = String::from_str("http://").unwrap();
        query.push_str(self.host);
        query.push_str(uri);
        query.push('?');
        query.push_str(&aws::canonical_query_string(& mut query_strings));
        let signature = aws::aws_s3_v2_sign(
            self.secrete_key, 
            &aws::aws_s3_v2_get_string_to_signed(method, uri, &mut signed_headers, payload)
        );
        let mut authorize_string = String::from_str("AWS ").unwrap();
        authorize_string.push_str(self.access_key);
        authorize_string.push(':');
        authorize_string.push_str(&signature);
        headers.insert(header::AUTHORIZATION, authorize_string.parse().unwrap());

        // get a client builder
        let client = Client::builder()
            .default_headers(headers)
            .build().unwrap();

        let action;
        match method {
            "GET" => {action = client.get(query.as_str());},
            "PUT" => {action = client.put(query.as_str());},
            "DELETE" => {action = client.delete(query.as_str());},
            _ => {
                error!("unspport HTTP verb");
                action = client.get(query.as_str());
            }
        }
        match action.body((*payload).clone()).send(){ // XXX fix payload as Vec<u8>
            Ok(mut res) => Ok(print_response(&mut res)),
            Err(_) => Err("Reqwest Error") //XXX
        }
    }
    fn aws_v4_request(&self, method: &str, virtural_host: Option<String>, uri: &str, qs: &Vec<(&str, &str)>, payload: Vec<u8>) -> Result<Vec<u8>, &'static str>{

        let utc: DateTime<Utc> = Utc::now();   
        let mut headers = header::HeaderMap::new();
        let time_str = utc.format("%Y%m%dT%H%M%SZ").to_string();
        headers.insert("x-amz-date", time_str.clone().parse().unwrap());

        let payload_hash = aws::hash_payload(&payload);
        headers.insert( "x-amz-content-sha256", payload_hash.parse().unwrap());

        let hostname = match virtural_host {
            Some(vs) => {
                let mut host = vs;
                host.push_str(".");
                host.push_str(self.host);
                host
            },
            None => {self.host.to_string()}
        };


        let mut signed_headers = vec![
            ("X-AMZ-Date", time_str.as_str()),
            ("Host", hostname.as_str())
        ];

        let mut query_strings = vec![];
        match self.format {
            Format::JSON => {query_strings.push(("format", "json"))}
            _ => {}
        }
        query_strings.extend(qs.iter().cloned());

        let mut query = String::from_str("http://").unwrap();
        query.push_str(hostname.as_str());
        query.push_str(uri);
        query.push('?');
        query.push_str(&aws::canonical_query_string(& mut query_strings));
        let signature = 
            aws::aws_v4_sign(self.secrete_key, 
                             aws::aws_v4_get_string_to_signed(
                                  method,
                                  uri,
                                  &mut query_strings,
                                  &mut signed_headers,
                                  &payload,
                                  utc.format("%Y%m%dT%H%M%SZ").to_string(),
                                  self.region.clone(),
                                  false).as_str(),
                              utc.format("%Y%m%d").to_string(),
                              self.region.clone(),
                              false);
        let mut authorize_string = String::from_str("AWS4-HMAC-SHA256 Credential=").unwrap();
        authorize_string.push_str(self.access_key);
        authorize_string.push('/');
        authorize_string.push_str(&format!("{}/{}/s3/aws4_request, SignedHeaders={}, Signature={}",
                                           utc.format("%Y%m%d").to_string(),
                                           self.region.clone().unwrap_or(String::from("us-east-1")),
                                           aws::signed_headers(&mut signed_headers), signature));
        headers.insert(header::AUTHORIZATION, authorize_string.parse().unwrap());

        // get a client builder
        let client = Client::builder()
            .default_headers(headers)
            .build().unwrap();

        let action;
        match method {
            "GET" => {action = client.get(query.as_str());},
            "PUT" => {action = client.put(query.as_str());},
            "DELETE" => {action = client.delete(query.as_str());},
            _ => {
                error!("unspport HTTP verb");
                action = client.get(query.as_str());
            }
        }
        match action.body(payload).send(){
            Ok(mut res) => Ok(print_response(&mut res)),
            Err(_) => Err("Reqwest Error") //XXX
        }
    }
    pub fn la(&self) -> Result<(), &'static str> {
        let re = Regex::new(RESPONSE_FORMAT).unwrap();
        let mut res: String;
        match self.auth_type {
            AuthType::AWS4 => { res = std::str::from_utf8(&try!(self.aws_v4_request("GET", None,"/", &Vec::new(), Vec::new()))).unwrap_or("").to_string();},
            AuthType::AWS2 => { res = std::str::from_utf8(&try!(self.aws_v2_request("GET", "/", &Vec::new(), &Vec::new()))).unwrap_or("").to_string();}
        }
        let result:serde_json::Value;
        let mut buckets = Vec::new();
        match self.format {
            Format::JSON => {
                result = serde_json::from_str(&res).unwrap();
                for bucket_list in  result[1].as_array(){
                    for bucket in bucket_list{
                        buckets.push(bucket["Name"].as_str().unwrap().to_string());
                    }
                }
            },
            Format::XML => {
                let mut reader = Reader::from_str(&res);
                let mut in_name_tag = false;
                let mut buf = Vec::new();

                loop {
                    match reader.read_event(&mut buf) {
                        Ok(Event::Start(ref e)) => {
                            if e.name() == b"Name" { in_name_tag = true; }
                        },
                        Ok(Event::End(ref e)) => {
                            if e.name() == b"Name" { in_name_tag = false; }
                        },
                        Ok(Event::Text(e)) => {
                            if in_name_tag { buckets.push(e.unescape_and_decode(&reader).unwrap()); }
                        },
                        Ok(Event::Eof) => break, 
                        Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                        _ => (), 
                    }
                    buf.clear();
                }
            }
        }
        for bucket in buckets{
            let bucket_prefix = format!("S3://{}", bucket.as_str());
            match self.auth_type {
                AuthType::AWS4 => { 
                    res = std::str::from_utf8(&try!(self.aws_v4_request("GET", None, &format!("/{}", bucket.as_str()), &Vec::new(), Vec::new()))).unwrap_or("").to_string();
                    match self.format {
                        Format::JSON => {
                            for cap in re.captures_iter(&res) {
                                println!("{}/{}", bucket_prefix, &cap[1]);
                            }
                        },
                        Format::XML => {
                            let mut reader = Reader::from_str(&res);
                            let mut in_key_tag = false;
                            let mut buf = Vec::new();

                            loop {
                                match reader.read_event(&mut buf) {
                                    Ok(Event::Start(ref e)) => { if e.name() == b"Key" { in_key_tag = true; } },
                                    Ok(Event::End(ref e)) => { if e.name() == b"Key" { in_key_tag = false; } },
                                    Ok(Event::Text(e)) => { if in_key_tag { println!("{}/{}", bucket_prefix, e.unescape_and_decode(&reader).unwrap()); }
                                    },
                                    Ok(Event::Eof) => break, 
                                    Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                                    _ => (), 
                                }
                                buf.clear();
                            }
                        }
                    }
                },
                AuthType::AWS2 => { 
                    res = std::str::from_utf8(&try!(self.aws_v2_request("GET", &format!("/{}", bucket.as_str()), &Vec::new(), &Vec::new()))).unwrap_or("").to_string();
                    match self.format {
                        Format::JSON => {
                            for cap in re.captures_iter(&res) {
                                println!("{}/{}", bucket_prefix, &cap[1]);
                            }
                        },
                        Format::XML => {
                            let mut reader = Reader::from_str(&res);
                            let mut in_key_tag = false;
                            let mut buf = Vec::new();

                            loop {
                                match reader.read_event(&mut buf) {
                                    Ok(Event::Start(ref e)) => { if e.name() == b"Key" { in_key_tag = true; } },
                                    Ok(Event::End(ref e)) => { if e.name() == b"Key" { in_key_tag = false; } },
                                    Ok(Event::Text(e)) => { if in_key_tag { println!("{}/{}", bucket_prefix, e.unescape_and_decode(&reader).unwrap()); }
                                    },
                                    Ok(Event::Eof) => break, 
                                    Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                                    _ => (), 
                                }
                                buf.clear();
                            }
                        }
                    }
                }
            }
        };
        Ok(())
    }

    pub fn ls(&self, bucket:Option<&str>)  -> Result<(), &'static str> {
        let res: String;
        match bucket {
            Some(b) => {
                let mut uri:String;
                let mut re = Regex::new(S3_FORMAT).unwrap();
                let mut vitural_host = None;
                if b.starts_with("s3://") || b.starts_with("S3://") {
                    let caps = re.captures(b).expect("S3 object format error.");
                    match  self.url_style {
                        UrlStyle::PATH => {
                            uri = format!("/{}", &caps["bucket"]);
                        },
                        UrlStyle::HOST=> {
                            vitural_host = Some(format!("{}", &caps["bucket"]));
                            uri = "/".to_string();
                        }
                    }
                } else {
                    uri = format!("/{}", b);
                }
                match self.auth_type {
                    AuthType::AWS4 => {res = std::str::from_utf8(&try!(self.aws_v4_request("GET", vitural_host.clone(), &uri, &Vec::new(), Vec::new()))).unwrap_or("").to_string();},
                    AuthType::AWS2 => {res = std::str::from_utf8(&try!(self.aws_v2_request("GET", &uri, &Vec::new(), &Vec::new()))).unwrap_or("").to_string();}
                }
                match self.format {
                    Format::JSON => {
                        re = Regex::new(RESPONSE_FORMAT).unwrap();
                        for cap in re.captures_iter(&res) {
                            println!("s3:/{}/{}", uri, &cap[1]);
                        }
                    },
                    Format::XML => {
                        let mut reader = Reader::from_str(&res);
                        let mut in_key_tag = false;
                        let mut buf = Vec::new();

                        loop {
                            match reader.read_event(&mut buf) {
                                Ok(Event::Start(ref e)) => {
                                    if e.name() == b"Key" { in_key_tag = true }
                                },
                                Ok(Event::End(ref e)) => {
                                    if e.name() == b"Key" { in_key_tag = false }
                                },
                                Ok(Event::Text(e)) => {
                                    if in_key_tag { println!("S3://{}/{} ", vitural_host.clone().unwrap_or(uri.to_string()),e.unescape_and_decode(&reader).unwrap()) }
                                },
                                Ok(Event::Eof) => break, 
                                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                                _ => (), 
                            }
                            buf.clear();
                        }
                    }
                }
            },
            None => {
                match self.auth_type {
                    AuthType::AWS4 => {res = std::str::from_utf8(&try!(self.aws_v4_request("GET", None, "/", &Vec::new(), Vec::new()))).unwrap_or("").to_string();},
                    AuthType::AWS2 => {res = std::str::from_utf8(&try!(self.aws_v2_request("GET", "/", &Vec::new(), &Vec::new()))).unwrap_or("").to_string();}
                }
                match self.format {
                    Format::JSON => {
                        let result:serde_json::Value = serde_json::from_str(&res).unwrap();
                        for bucket_list in  result[1].as_array(){
                            for bucket in bucket_list{
                                println!("S3://{} ", bucket["Name"].as_str().unwrap());
                            }
                        }
                    },
                    Format::XML => {
                        let mut reader = Reader::from_str(&res);
                        let mut in_name_tag = false;
                        let mut buf = Vec::new();

                        loop {
                            match reader.read_event(&mut buf) {
                                Ok(Event::Start(ref e)) => {
                                    if e.name() == b"Name" { in_name_tag = true }
                                },
                                Ok(Event::End(ref e)) => {
                                    if e.name() == b"Name" { in_name_tag = false }
                                },
                                Ok(Event::Text(e)) => {
                                    if in_name_tag { println!("S3://{} ", e.unescape_and_decode(&reader).unwrap()) }
                                },
                                Ok(Event::Eof) => break, 
                                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                                _ => (), 
                            }
                            buf.clear();
                        }
                    }
                }
            } 
        };
        Ok(())
    }

    pub fn put(&self, file:&str, dest:&str) -> Result<(), &'static str> {
        if file == "" || dest == "" {
            return Err("please specify the file and the destiney")
        }
        let re = Regex::new(S3_FORMAT).unwrap();
        let caps = match re.captures(dest){
            Some(c) => c,
            None => return Err("S3 object format error.")
        };
        let mut content: Vec<u8>;


        if !Path::new(file).exists() && file == "test"{ 
            content = vec![83, 51, 82, 83, 32, 116, 101, 115, 116, 10];  // S3RS test/n
        } else {
            let mut fin = match File::open(file) {
                Ok(f) => f,
                Err(_) => return Err("input file open error")
            };
            content = Vec::new();
            let _ = fin.read_to_end(&mut content);
        }

        let uri = if &caps["object"] == "" || &caps["object"] == "/" {
            let file_name =  Path::new(file).file_name().unwrap().to_string_lossy();
            format!("/{}/{}", &caps["bucket"], file_name)
        } else {
            format!("/{}{}", &caps["bucket"], &caps["object"])
        };

        match self.auth_type {
            AuthType::AWS4 => {try!(self.aws_v4_request("PUT", None, &uri, &Vec::new(), content));},
            AuthType::AWS2 => {try!(self.aws_v2_request("PUT", &uri, &Vec::new(), &content));}
        };
        Ok(())
    }

    pub fn get(&self, src:&str, file:Option<&str>) -> Result<(), &'static str> {
        if src == "" { return Err("Please specify the object")}
        let re = Regex::new(S3_FORMAT).unwrap();
        let mut virtural_host = None;
        let uri:String;
        let caps = match re.captures(src) {
            Some(c) => c,
            None => return Err("S3 object format error.")
        };

        let fout = match file {
            Some(fname) => fname,
            None => {
                Path::new(src).file_name().unwrap().to_str().unwrap_or("s3download")
            }
        };

        if &caps["object"] == ""{
            return Err("Please specific the object")
        }

        match self.auth_type {
            AuthType::AWS4 => {
                match  self.url_style {
                    UrlStyle::PATH => {
                        uri = format!("/{}{}", &caps["bucket"], &caps["object"]);
                    },
                    UrlStyle::HOST=> {
                        virtural_host = Some(format!("{}", &caps["bucket"]));
                        uri = format!("{}", &caps["object"]);
                    }
                }
                match write(fout, try!(self.aws_v4_request("GET", virtural_host, &uri, &Vec::new(), Vec::new()))){
                    Ok(_) => return Ok(()),
                    Err(_) => return Err("write file error") //XXX
                }
            },
            AuthType::AWS2 => {
                match write(fout, try!(self.aws_v2_request("GET", &format!("/{}{}", &caps["bucket"], &caps["object"]), &Vec::new(), &Vec::new()))){
                    Ok(_) => return Ok(()),
                    Err(_) => return Err("write file error") //XXX
                }
            }
        }
    }

    pub fn cat(&self, src:&str) -> Result<(), &'static str> {
        if src == "" {return Err("please specific the object")}
        let re = Regex::new(S3_FORMAT).unwrap();
        let mut virtural_host = None;
        let uri:String;
        let caps = match re.captures(src) {
            Some(c) => c,
            None => return Err("S3 object format error.")
        };

        if &caps["object"] == ""{
            return Err("Please specific the object")
        }

        match self.auth_type {
            AuthType::AWS4 => {
                match  self.url_style {
                    UrlStyle::PATH => {
                        uri = format!("/{}{}", &caps["bucket"], &caps["object"]);
                    },
                    UrlStyle::HOST=> {
                        virtural_host = Some(format!("{}", &caps["bucket"]));
                        uri = format!("{}", &caps["object"]);
                    }
                }
                match self.aws_v4_request("GET", virtural_host, &uri, &Vec::new(), Vec::new()){
                    Ok(b) => { println!("{}", std::str::from_utf8(&b).unwrap_or("")); return Ok(()) },
                    Err(e) => return Err(e) 
                }
            },
            AuthType::AWS2 => {
                match self.aws_v2_request("GET", &format!("/{}{}", &caps["bucket"], &caps["object"]), &Vec::new(), &Vec::new()){
                    Ok(b) => { println!("{}", std::str::from_utf8(&b).unwrap_or("")); return Ok(()) },
                    Err(e) => return Err(e) 
                }
            }
        }
    }

    pub fn del(&self, src:&str) -> Result<(), &'static str> {
        if src == "" {return Err("please specific the object")}
        let re = Regex::new(S3_FORMAT).unwrap();
        let mut virtural_host = None;
        let uri:String;
        let caps = match re.captures(src) {
            Some(c) => c,
            None => return Err("S3 object format error.")
        };

        if &caps["object"] == ""{
            return Err("Please specific the object")
        }
        match  self.url_style {
            UrlStyle::PATH => {
                uri = format!("/{}{}", &caps["bucket"], &caps["object"]);
            },
            UrlStyle::HOST=> {
                virtural_host = Some(format!("{}", &caps["bucket"]));
                uri = format!("{}", &caps["object"]);
            }
        }

        match self.auth_type {
            AuthType::AWS4 => {try!(self.aws_v4_request("DELETE", virtural_host, &uri, &Vec::new(), Vec::new()));},
            AuthType::AWS2 => {try!(self.aws_v2_request("GET", &format!("/{}{}", &caps["bucket"], &caps["object"]), &Vec::new(), &Vec::new()));}
        }
        Ok(())
    }

    pub fn mb(&self, bucket: &str) -> Result<(), &'static str> {
        if bucket == "" {return Err("please specific the bucket name")}
        let mut uri = String::from_str("/").unwrap();
        uri.push_str(bucket);
        match self.auth_type {
            AuthType::AWS4 => {try!(self.aws_v4_request("PUT", None, &uri, &Vec::new(), Vec::new()));},
            AuthType::AWS2 => {try!(self.aws_v2_request("PUT", &uri, &Vec::new(), &Vec::new()));}
        };
        Ok(())
    }

    pub fn rb(&self, bucket: &str) -> Result<(), &'static str> {
        if bucket == "" {return Err("please specific the bucket name")}
        let mut uri = String::from_str("/").unwrap();
        uri.push_str(bucket);
        match self.auth_type {
            AuthType::AWS4 => {try!(self.aws_v4_request("DELETE", None, &uri, &Vec::new(), Vec::new()));},
            AuthType::AWS2 => {try!(self.aws_v2_request("DELETE", &uri, &Vec::new(), &Vec::new()));}
        };
        Ok(())
    }

    pub fn tag(&self, target: &str, tags: &Vec<(&str, &str)>) -> Result<(), &'static str>  {
        debug!("target: {:?}", target);
        debug!("tags: {:?}", tags);
        if target == "" {return Err("please specific the object")}
        let re = Regex::new(S3_FORMAT).unwrap();
        let mut virtural_host = None;
        let uri:String;
        let caps = match re.captures(target) {
            Some(c) => c,
            None => return Err("S3 object format error.")
        };

        if &caps["object"] == ""{
            return Err("Please specific the object")
        }
        match  self.url_style {
            UrlStyle::PATH => {
                uri = format!("/{}{}", &caps["bucket"], &caps["object"]);
            },
            UrlStyle::HOST=> {
                virtural_host = Some(format!("{}", &caps["bucket"]));
                uri = format!("{}", &caps["object"]);
            }
        }
        let mut content = format!("<Tagging><TagSet>");
        for tag in tags {
            content.push_str(&format!("<Tag><Key>{}</Key><Value>{}</Value></Tag>", tag.0, tag.1));
        };
        content.push_str(&format!("</TagSet></Tagging>"));
        debug!("payload: {:?}", content);

        let query_string = vec![("tagging", "")];
        match self.auth_type {
            AuthType::AWS4 => {try!(self.aws_v4_request("PUT", virtural_host, &uri, &query_string, content.into_bytes()));},
            AuthType::AWS2 => {try!(self.aws_v2_request("PUT", &format!("/{}{}", &caps["bucket"], &caps["object"]), &query_string, &content.into_bytes()));}
        }
        Ok(())
    }

    pub fn url_command(&self, url: &str) -> Result<(), &'static str>  {
        let mut uri = String::new();
        let mut raw_qs = String::new();
        let mut query_strings = Vec::new();
        match url.find('?'){
            Some(idx) =>{
                uri.push_str(&url[..idx]);
                raw_qs.push_str(&String::from_str(&url[idx+1..]).unwrap());
                for q_pair in raw_qs.split('&'){
                    match q_pair.find('='){
                        Some(_)=>{query_strings.push((q_pair.split('=').nth(0).unwrap(),
                                                     q_pair.split('=').nth(1).unwrap()))},
                        None => {query_strings.push((&q_pair, ""))}
                    }
                }
            },
            None => {
                uri.push_str(&url);
            }
        }

        let result = match self.auth_type {
            AuthType::AWS4 => {try!(self.aws_v4_request("GET", None, &uri, &query_strings, Vec::new()))},
            AuthType::AWS2 => {try!(self.aws_v2_request("GET", &uri, &query_strings, &Vec::new()))}
        };
        println!("{}", std::str::from_utf8(&result).unwrap_or(""));
        Ok(())
    }

    pub fn change_s3_type(&mut self, command: &str){
        println!("set up s3 type as {}", command);
        if command.ends_with("aws"){
            self.auth_type = AuthType::AWS4;
            self.format = Format::XML;
            self.url_style = UrlStyle::HOST;
            println!("using aws verion 4 protocol, xml format, and host style url");
        } else if command.ends_with("ceph") {
            self.auth_type = AuthType::AWS4;
            self.format = Format::JSON;
            self.url_style = UrlStyle::PATH;
            println!("using aws verion 4 protocol, json format, and path style url");
        }else{
            println!("usage: s3_type [aws/ceph]");
        }
    }

    pub fn change_auth_type(&mut self, command: &str){
        if command.ends_with("aws2"){
            self.auth_type = AuthType::AWS2;
            println!("using aws version 2 protocol");
        } else if command.ends_with("aws4") || command.ends_with("aws") {
            self.auth_type = AuthType::AWS4;
            println!("using aws verion 4 protocol");
        }else{
            println!("usage: auth_type [aws4/aws2]");
        }
    }

    pub fn change_format_type(&mut self, command: &str){
        if command.ends_with("xml"){
            self.format = Format::XML;
            println!("using xml format");
        } else if command.ends_with("json") {
            self.format = Format::JSON;
            println!("using json format");
        }else{
            println!("usage: format_type [xml/json]");
        }
    }

    pub fn change_url_style(&mut self, command: &str){
        if command.ends_with("path"){
            self.url_style = UrlStyle::PATH;
            println!("using path style url");
        } else if command.ends_with("host") {
            self.url_style = UrlStyle::HOST;
            println!("using host style url");
        }else{
            println!("usage: url_style [path/host]");
        }

    }
}
