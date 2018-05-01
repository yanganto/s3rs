use std::str::FromStr;
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use sha2::Sha256 as sha2_256;
use hmac::{Hmac, Mac};
use rustc_serialize::hex::ToHex;
use base64::encode;
use url::form_urlencoded;

fn canonical_query_string(query_strings:&mut Vec<(&str, &str)>) -> String {
    query_strings.sort_by_key(|a| a.0);
    let mut encoded = form_urlencoded::Serializer::new(String::new());
    for q in query_strings{
        encoded.append_pair(q.0, q.1);
    }
    encoded.finish()
}

//CanonicalHeaders = CanonicalHeadersEntry0 + CanonicalHeadersEntry1 + ... + CanonicalHeadersEntryN
//CanonicalHeadersEntry = Lowercase(HeaderName) + ':' + Trimall(HeaderValue) + '\n'
fn canonical_headers(headers:&mut Vec<(&str, &str)>) -> String {
    let mut output = String::new();
    headers.sort_by(|a, b| a.0.to_lowercase().as_str().cmp(b.0.to_lowercase().as_str()));
    for h in headers {
        output.push_str(h.0.to_lowercase().as_str());
        output.push(':');
        output.push_str(h.1.trim());
        output.push('\n');
    }
    output
}

//SignedHeaders = Lowercase(HeaderName0) + ';' + Lowercase(HeaderName1) + ";" + ... + Lowercase(HeaderNameN)
pub fn signed_headers(headers:&mut Vec<(&str, &str)>) -> String {
    let mut output = Vec::new();
    headers.sort_by(|a, b| a.0.to_lowercase().as_str().cmp(b.0.to_lowercase().as_str()));
    for h in headers {
        output.push(h.0.to_lowercase());
    }
    output.join(";")
}

//HashedPayload = Lowercase(HexEncode(Hash(requestPayload)))
fn hash_payload(payload: &str) -> String {
    let mut sha = Sha256::new();
    sha.input_str(payload);
    trace!("payload request hash = {}", sha.result_str());
    sha.result_str()
}




fn aws_v4_canonical_request(http_method: &str, uri:&str, query_strings:&mut Vec<(&str, &str)>, headers:&mut Vec<(&str, &str)>, payload:&str) -> String {
    let mut input = String::new();
    input.push_str(http_method);
    input.push_str("\n");
    input.push_str(uri);
    input.push_str("\n");
    input.push_str(canonical_query_string(query_strings).as_str());
    input.push_str("\n");
    input.push_str(canonical_headers(headers).as_str());
    input.push_str("\n");
    input.push_str(signed_headers(headers).as_str());
    input.push_str("\n");
    input.push_str(hash_payload(payload).as_str());

    trace!("canonical request:\n{}", input);
    
    let mut sha = Sha256::new();
    sha.input_str(input.as_str());
    trace!("canonical request hash = {}", sha.result_str());
    sha.result_str()
}

pub fn aws_v4_get_string_to_signed(http_method: &str, uri:&str,  query_strings:&mut Vec<(&str, &str)>, headers:&mut Vec<(&str, &str)>, payload:&str, time_str:String) -> String {
    let mut string_to_signed = String::from_str("AWS4-HMAC-SHA256\n").unwrap();
    string_to_signed.push_str(&time_str);
    string_to_signed.push_str("\n");
    unsafe{
        string_to_signed.push_str(&format!("{}/us-east-1/iam/aws4_request", time_str.slice_unchecked(0,8)));
    }
    string_to_signed.push_str("\n");
    string_to_signed.push_str(aws_v4_canonical_request(http_method, uri, query_strings, headers, payload).as_str());
    trace!("string_to_signed:\n{}", string_to_signed);
    return  string_to_signed
}


// HMAC(HMAC(HMAC(HMAC("AWS4" + kSecret,"20150830"),"us-east-1"),"iam"),"aws4_request")
pub fn aws_v4_sign(secret_key: &str, data: &str, time_str: String) -> String {
    let mut key = String::from("AWS4");
    key.push_str(secret_key);

    let mut mac = Hmac::<sha2_256>::new(key.as_str().as_bytes());
    mac.input(time_str.as_str().as_bytes());
    let result = mac.result();
    let code_bytes = result.code();
    trace!("date_k = {}", code_bytes.to_hex());

    let mut mac1 = Hmac::<sha2_256>::new(code_bytes);
    mac1.input(b"us-east-1");
    let result1 = mac1.result();
    let code_bytes1 = result1.code();
    trace!("region_k = {}", code_bytes1.to_hex());

    let mut mac2 = Hmac::<sha2_256>::new(code_bytes1);
    mac2.input(b"iam");
    let result2 = mac2.result();
    let code_bytes2 = result2.code();
    trace!("service_k = {}", code_bytes2.to_hex());

    let mut mac3 = Hmac::<sha2_256>::new(code_bytes2);
    mac3.input(b"aws4_request");
    let result3 = mac3.result();
    let code_bytes3 = result3.code();
    trace!("signing_k = {}", code_bytes3.to_hex());

    let mut mac4 = Hmac::<sha2_256>::new(code_bytes3);
    mac4.input(data.as_bytes());
    let result4 = mac4.result();
    let code_bytes4 = result4.code();
    trace!("signature = {}", code_bytes4.to_hex());

    code_bytes4.to_hex()
}

pub fn aws_v2_get_string_to_signed(http_method: &str, host:&str, uri:&str, query_strings:&mut Vec<(&str, &str)>) -> String {
    let mut string_to_signed = String::from_str(http_method).unwrap();
    string_to_signed.push_str("\n");
    string_to_signed.push_str(host);
    string_to_signed.push_str("\n");
    string_to_signed.push_str(uri);
    string_to_signed.push_str("\n");
    string_to_signed.push_str(canonical_query_string(query_strings).as_str());
    return  string_to_signed
}

pub fn aws_v2_sign(secret_key: &str, data: &str) -> String {
    let mut mac = Hmac::<sha2_256>::new(secret_key.as_bytes());
    mac.input(data.as_bytes());

    let result = mac.result();
    let code_bytes = result.code();

    encode(code_bytes)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_v2_get_string_to_signed() {
        let mut query_strings = vec![
            ("Timestamp", "2011-10-03T15:19:30"),
            ("AWSAccessKeyId", "AKIAIOSFODNN7EXAMPLE"),
            ("Action", "DescribeJobFlows"),
            ("SignatureMethod", "HmacSHA256"),
            ("SignatureVersion", "2"),
            ("Version", "2009-03-31")
        ];

        let string_need_signed = aws_v2_get_string_to_signed(
            "GET",
            "elasticmapreduce.amazonaws.com",
            "/", 
            &mut query_strings);

        assert_eq!(
            "GET\n\
            elasticmapreduce.amazonaws.com\n\
            /\n\
            AWSAccessKeyId=AKIAIOSFODNN7EXAMPLE&\
            Action=DescribeJobFlows&\
            SignatureMethod=HmacSHA256&\
            SignatureVersion=2&\
            Timestamp=2011-10-03T15%3A19%3A30\
            &Version=2009-03-31", string_need_signed.as_str());
    }

    #[test]
    fn test_aws_v2_sign() {
        let sig = aws_v2_sign("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY", 
                              "GET\n\
                              elasticmapreduce.amazonaws.com\n\
                              /\n\
                              AWSAccessKeyId=AKIAIOSFODNN7EXAMPLE&\
                              Action=DescribeJobFlows&\
                              SignatureMethod=HmacSHA256&\
                              SignatureVersion=2&\
                              Timestamp=2011-10-03T15%3A19%3A30&\
                              Version=2009-03-31");
        assert_eq!("i91nKc4PWAt0JJIdXwz9HxZCJDdiy6cf/Mj6vPxyYIs=", sig.as_str());
    }
    #[test]
    fn test_hash_payload() {
        assert_eq!(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            hash_payload(""));
    }

    #[test]
    fn test_aws_v4_get_string_to_signed() {
        let mut headers = vec![
            ("X-AMZ-Date", "20150830T123600Z"),
            ("Host", "iam.amazonaws.com"),
            ("Content-Type", "application/x-www-form-urlencoded; charset=utf-8")
        ];

        let mut query_strings = vec![
            ("Version", "2010-05-08"),
            ("Action", "ListUsers")
        ];

        let string_need_signed = aws_v4_get_string_to_signed(
                "GET",
                "/", 
                &mut query_strings, 
                &mut headers,
                "",
                "20150830T123600Z".to_string());

        assert_eq!(
            "AWS4-HMAC-SHA256\n\
            20150830T123600Z\n\
            20150830/us-east-1/iam/aws4_request\n\
            f536975d06c0309214f805bb90ccff089219ecd68b2577efef23edd43b7e1a59",
            string_need_signed.as_str());
    }


    #[test]
    fn test_aws_v4_sign() {
        let sig = aws_v4_sign("wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY", 
                              "AWS4-HMAC-SHA256\n\
                              20150830T123600Z\n\
                              20150830/us-east-1/iam/aws4_request\n\
                              f536975d06c0309214f805bb90ccff089219ecd68b2577efef23edd43b7e1a59",
                              "20150830".to_string()
                              );

        assert_eq!("5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7", sig.as_str());
    }



}
