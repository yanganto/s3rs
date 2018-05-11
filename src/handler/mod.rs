use chrono::prelude::*;
use reqwest::{Response, header, Client, StatusCode};
use std::str::FromStr;
use serde_json;
use regex::Regex;


mod aws;

pub enum S3Type{
    AWS4,
    AWS2,
    // OSS
}


pub struct Handler<'a>{
    pub host: &'a str,
    pub access_key: &'a str,
    pub secrete_key: &'a str,
    pub s3_type: S3Type
}


pub trait S3 {
    fn la(&self) -> Response;
}

fn print_response(res: &mut Response) -> String{
    let body = res.text().unwrap_or("".to_string());
    if res.status() != StatusCode::Ok{
        println!("Status: {}", res.status());
        println!("Headers:\n{}", res.headers());
        println!("Body:\n{}\n\n", &body);
    } else {
        info!("Status: {}", res.status());
        info!("Headers:\n{}", res.headers());
        info!("Body:\n{}\n\n", &body);
    }
    body 
}

impl<'a> Handler<'a>  {
    fn aws_v2_request(&self, method: &str, uri: &str, qs: &Vec<(&str, &str)>, payload: &str) -> String {

        let utc: DateTime<Utc> = Utc::now();   
        header! { (Date, "date") => [String] }
        let mut headers = header::Headers::new();
        let time_str = utc.to_rfc2822();
        headers.set(Date(time_str.clone()));

        // NOTE: ceph has bug using x-amz-date
        let mut signed_headers = vec![
            ("Date", time_str.as_str())
        ];

        let mut query_strings = vec![
            ("format", "json")
        ];
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
        headers.set(header::Authorization(authorize_string));

        // get a client builder
        let client = Client::builder()
            .default_headers(headers)
            .build().unwrap();

        let mut res: Response;
        match method {
            "GET" => {res = client.get(query.as_str()).send().unwrap();},
            "PUT" => {res = client.put(query.as_str()).send().unwrap();},
            "DELETE" => {res = client.delete(query.as_str()).send().unwrap();},
            _ => {
                error!("unspport HTTP verb");
                res = client.get(query.as_str()).send().unwrap();
            }
        }
        print_response(&mut res)
    }
    fn aws_v4_request(&self, method: &str, uri: &str, qs: &Vec<(&str, &str)>, payload: &str) -> String{

        let utc: DateTime<Utc> = Utc::now();   
        header! { (XAMZDate, "x-amz-date") => [String] }
        let mut headers = header::Headers::new();
        let time_str = utc.format("%Y%m%dT%H%M%SZ").to_string();
        headers.set(XAMZDate(time_str.clone()));

        let mut signed_headers = vec![
            ("X-AMZ-Date", time_str.as_str()),
            ("Host",self.host)
        ];

        let mut query_strings = vec![
            ("format", "json")
        ];
        query_strings.extend(qs.iter().cloned());

        let mut query = String::from_str("http://").unwrap();
        query.push_str(self.host);
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
                                  payload,
                                  utc.format("%Y%m%dT%H%M%SZ").to_string()).as_str(),
                              utc.format("%Y%m%d").to_string());
        let mut authorize_string = String::from_str("AWS4-HMAC-SHA256 Credential=").unwrap();
        authorize_string.push_str(self.access_key);
        authorize_string.push('/');
        authorize_string.push_str(&format!("{}/us-east-1/iam/aws4_request, SignedHeaders={}, Signature={}",
                                           utc.format("%Y%m%d").to_string(),
                                           aws::signed_headers(&mut signed_headers), signature));
        headers.set(header::Authorization(authorize_string));

        // get a client builder
        let client = Client::builder()
            .default_headers(headers)
            .build().unwrap();

        let mut res: Response;
        match method {
            "GET" => {res = client.get(query.as_str()).send().unwrap();},
            "PUT" => {res = client.put(query.as_str()).send().unwrap();},
            "DELETE" => {res = client.delete(query.as_str()).send().unwrap();},
            _ => {
                error!("unspport HTTP verb");
                res = client.get(query.as_str()).send().unwrap();
            }
        }
        print_response(&mut res)
    }
    pub fn la(&self) {
        let re = Regex::new(r#""Contents":\["([A-Za-z0-9.]+?)"(.*?)\]"#).unwrap();
        let mut res: String;
        match self.s3_type {
            S3Type::AWS4 => { res = self.aws_v4_request("GET", "/", &Vec::new(),"");},
            S3Type::AWS2 => { res = self.aws_v2_request("GET", "/", &Vec::new(),"");}
        }
        let result:serde_json::Value = serde_json::from_str(&res).unwrap();
        for bucket_list in  result[1].as_array(){
            for bucket in bucket_list{
                let bucket_prefix = format!("S3://{}", bucket["Name"].as_str().unwrap());
                match self.s3_type {
                    S3Type::AWS4 => { 
                        res = self.aws_v4_request("GET", &format!("/{}", bucket["Name"].as_str().unwrap()), &Vec::new(),"");
                        for cap in re.captures_iter(&res) {
                            println!("{}/{}", bucket_prefix, &cap[1]);
                        }
                    },
                    S3Type::AWS2 => { 
                        res = self.aws_v2_request("GET", &format!("/{}", bucket["Name"].as_str().unwrap()), &Vec::new(),"");
                        for cap in re.captures_iter(&res) {
                            println!("{}/{}", bucket_prefix, &cap[1]);
                        }
                    }
                }
            }
        }
    }

    pub fn ls(&self, bucket:Option<&str>) {
        let res: String;
        if bucket.is_some() {
            let re = Regex::new(r#""Contents":\["([A-Za-z0-9.]+?)"(.*?)\]"#).unwrap();
            match self.s3_type {
                S3Type::AWS4 => {res = self.aws_v4_request("GET", &format!("/{}", bucket.unwrap()), &Vec::new(),"");},
                S3Type::AWS2 => {res = self.aws_v2_request("GET", &format!("/{}", bucket.unwrap()), &Vec::new(),"");}
            }
            for cap in re.captures_iter(&res) {
                println!("s3://{}/{}", bucket.unwrap(), &cap[1]);
            }
        } else {
            match self.s3_type {
                S3Type::AWS4 => {res = self.aws_v4_request("GET", "/", &Vec::new(),"");},
                S3Type::AWS2 => {res = self.aws_v2_request("GET", "/", &Vec::new(),"");}
            }
            let result:serde_json::Value = serde_json::from_str(&res).unwrap();
            for bucket_list in  result[1].as_array(){
                for bucket in bucket_list{
                    println!("S3://{} ", bucket["Name"].as_str().unwrap());
                }
            }
        }
    }

    pub fn mb(&self, bucket: &str) {
        let mut uri = String::from_str("/").unwrap();
        uri.push_str(bucket);
        match self.s3_type {
            S3Type::AWS4 => {self.aws_v4_request("PUT", &uri, &Vec::new(),"");},
            S3Type::AWS2 => {self.aws_v2_request("PUT", &uri, &Vec::new(),"");}
        }
    }

    pub fn rb(&self, bucket: &str) {
        let mut uri = String::from_str("/").unwrap();
        uri.push_str(bucket);
        match self.s3_type {
            S3Type::AWS4 => {self.aws_v4_request("DELETE", &uri, &Vec::new(),"");},
            S3Type::AWS2 => {self.aws_v2_request("DELETE", &uri, &Vec::new(),"");}
        }
    }

    pub fn url_command(&self, url: &str) {
        let mut uri = String::new();
        let mut raw_qs = String::new();
        let mut query_strings = Vec::new();
        match url.find('?'){
            Some(idx) =>{
                uri.push_str(&url[..idx]);
                raw_qs.push_str(&String::from_str(&url[idx+1..]).unwrap());
                for q_pair in raw_qs.split('&'){
                    match q_pair.find('='){
                        Some(i)=>{query_strings.push(q_pair.split_at(i))},
                        None => {query_strings.push((&q_pair, ""))}
                    }
                }
            },
            None => {
                uri.push_str(&url);
            }
        }

        match self.s3_type {
            S3Type::AWS4 => {self.aws_v4_request("GET", &uri, &query_strings,"");},
            S3Type::AWS2 => {self.aws_v2_request("GET", &uri, &query_strings,"");}
        }
    }

}
