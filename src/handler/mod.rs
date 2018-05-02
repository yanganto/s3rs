use chrono::prelude::*;
use reqwest::{Response, header, Client};
use std::str::FromStr;

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


impl<'a> Handler<'a>  {
    fn aws_v2_request(&self, method: &str, uri: &str, qs: &Vec<(&str, &str)>, payload: &str) -> Response{

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
        match method {
            "GET" => {
                client.get(query.as_str()).send().unwrap()
            }
            _ => {
                error!("unspport HTTP verb");
                client.get(query.as_str()).send().unwrap()
            }
        }

    }
    fn aws_v4_request(&self, method: &str, uri: &str, qs: &Vec<(&str, &str)>, payload: &str) -> Response{

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
        match method {
            "GET" => {
                client.get(query.as_str()).send().unwrap()
            }
            _ => {
                error!("unspport HTTP verb");
                client.get(query.as_str()).send().unwrap()
            }
        }
    }
    pub fn la(&self) -> Response{
        match self.s3_type {
            S3Type::AWS4 => {
                self.aws_v4_request("GET", "/", &Vec::new(),"")
            },
            S3Type::AWS2 =>{
                self.aws_v2_request("GET", "/", &Vec::new(),"")
            }
        }
    }

    pub fn url_command(&self, url: &str) -> Response{
        let mut uri = String::new();
        let mut raw_qs = String::new();
        let mut query_strings = Vec::new();
        match url.find('?'){
            Some(idx) =>{
                uri.push_str(&url[..idx]);
                raw_qs.push_str(&String::from_str(&url[idx+1..]).unwrap());
                for q_pair in raw_qs.split('&'){
                    match q_pair.find('='){
                        Some(i)=>{query_strings.push((&q_pair[..i], &q_pair[i+1..]))},
                        None => {query_strings.push((&q_pair, ""))}
                    }
                }
            },
            None => {
                uri.push_str(&url);
            }
        }


        match self.s3_type {
            S3Type::AWS4 => {
                self.aws_v4_request("GET", &uri, &query_strings,"")
            },
            S3Type::AWS2 =>{
                self.aws_v2_request("GET", &uri, &query_strings,"")
            }
        }
    }

}
