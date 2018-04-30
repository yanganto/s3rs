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


use std::io;
use std::io::{Read, Write, BufReader, BufRead};
use std::fs::{File, OpenOptions};
use std::str;
use std::str::FromStr;
use std::io::stdout;
use reqwest::header;
use hyper::header::Headers;
use std::time::SystemTime;
use chrono::prelude::*;
// use sha2::{Digest, Sha256};
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use sha2::Sha256 as sha2_256;
use hmac::{Hmac, Mac};
use rustc_serialize::hex::ToHex;




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

fn v4_hash_canonical_request(http_method: &str, uri:&str, query_string:&str, headers:&Vec<String>, signed_headers:&str, payload_hash:&str) -> String {
    let mut input = String::new();
    input.push_str(http_method);
    input.push_str("\n");
    input.push_str(uri);
    input.push_str("\n");
    input.push_str(query_string);
    input.push_str("\n");
    for h in headers {
        input.push_str(h.as_str());
        input.push_str("\n");
    }
    input.push_str("\n");
    input.push_str(signed_headers);
    input.push_str("\n");
    input.push_str(payload_hash);

    let mut sha = Sha256::new();
    sha.input_str(input.as_str());
    sha.result_str()
}

fn v4_get_string_to_signed(http_method: &str, uri:&str, query_string:&str, headers:&Vec<String>, signed_headers:&str, payload_hash:&str) -> String {
    let mut string_to_signed = String::from_str("AWS4-HMAC-SHA256\n").unwrap();
    string_to_signed.push_str("20150830T123600Z");
    string_to_signed.push_str("\n");
    string_to_signed.push_str("20150830/us-east-1/iam/aws4_request");
    string_to_signed.push_str("\n");
    string_to_signed.push_str(v4_hash_canonical_request(http_method, uri, query_string, headers, signed_headers, payload_hash).as_str());
    return  string_to_signed
}


// HMAC(HMAC(HMAC(HMAC("AWS4" + kSecret,"20150830"),"us-east-1"),"iam"),"aws4_request")
fn v4_sign(kSecret: &str, data: &str) {
    let mut key = String::from("AWS4");
    key.push_str(kSecret);
    let mut mac = Hmac::<sha2_256>::new(key.as_str().as_bytes());
    mac.input(b"20150830");

    // `result` has type `MacResult` which is a thin wrapper around array of
    // bytes for providing constant time equality check
    let result = mac.result();
    // To get underlying array use `code` method, but be carefull, since
    // incorrect use of the code value may permit timing attacks which defeat
    // the security provided by the `MacResult`
    let code_bytes = result.code();

    let mut mac1 = Hmac::<sha2_256>::new(code_bytes);
    mac1.input(b"us-east-1");
    let result1 = mac1.result();
    let code_bytes1 = result1.code();

    let mut mac2 = Hmac::<sha2_256>::new(code_bytes1);
    mac2.input(b"iam");
    let result2 = mac2.result();
    let code_bytes2 = result2.code();

    let mut mac3 = Hmac::<sha2_256>::new(code_bytes2);
    mac3.input(b"aws4_request");
    let result3 = mac3.result();
    let code_bytes3 = result3.code();

    let mut mac4 = Hmac::<sha2_256>::new(code_bytes3);
    mac4.input(data.as_bytes());
    let result4 = mac4.result();
    let code_bytes4 = result4.code();

    println!("sig: {:?}", code_bytes4.to_hex());
}

		
fn main() {

    println!("////////// TEST AWS4  //////////");
    let h = vec![String::from_str("content-type:application/x-www-form-urlencoded; charset=utf-8").unwrap(), 
                String::from_str("host:iam.amazonaws.com").unwrap(), String::from_str("x-amz-date:20150830T123600Z").unwrap()];
    let string_need_signed = v4_get_string_to_signed(
            "GET",
            "/", 
            "Action=ListUsers&Version=2010-05-08", 
            &h,
            "content-type;host;x-amz-date", 
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");

    println!("string need signed: \n{}\n", string_need_signed);
    v4_sign("wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY", 
          string_need_signed.as_str());

    println!("5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7");
    println!("////////////////////////////////");




    let verbose = true;

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
    if verbose {
        println!("host: {}", credential.host);
        println!("access key: {}", credential.access_key);
        println!("secrete key: {}", credential.secrete_key);
    }

    println!("enter command, help for usage or exit for quit");

    let mut raw_input;
    let mut command = String::new(); 
    let mut res;
    while command != "exit" {
        print!("> ");
        stdout().flush();

        raw_input = read_from_tty(|_buf, b, tty| {
            tty.write(&[b]);
        }, false, false).unwrap();
        command = String::from_utf8(raw_input).unwrap();
        println!("");
        if verbose {
            println!("===== do command: {:?} =====", command);
        }
        if command.starts_with("la"){


            // XXX: Implement AWS4 auth here
            let mut query = String::from_str("http://").unwrap();
            query.push_str(credential.host.as_str());
            // query.push_str("www.rust-lang.org");
            // query.push_str("?format=json");

            let mut headers = header::Headers::new();

            let utc: DateTime<Utc> = Utc::now();   
            // let utc: DateTime<Utc> = Utc.ymd(2018, 04, 29).and_hms(4, 45, 1);
            header! { (XAMZDate, "x-amz-date") => [String] }
            headers.set(XAMZDate(utc.to_rfc2822()));


            // let mut mac = Hmac::<Sha256>::new(
            //         &base64::decode(credential.secrete_key.as_str()).expect("secrete key decode error")
            //     ).unwrap();
            // mac.input(b"");
            // let result = mac.result();
            // let code_bytes = result.code();

            // println!("auth: {:?}", base64::encode(&code_bytes));

            let mut authorize_string = String::from_str("AWS ").unwrap();
            authorize_string.push_str(credential.access_key.as_str());
            authorize_string.push(':');
            // authorize_string.push_str(base64::encode(&code_bytes).as_str());
            // headers.set(header::Authorization(authorize_string));


            // get a client builder
            let client = reqwest::Client::builder()
                .default_headers(headers)
                .build().unwrap();

            res = client.get(query.as_str()).send().unwrap();

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
        stdout().flush();
    }
}
