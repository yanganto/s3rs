use colored::{self, *};
pub mod secret;

pub fn print_usage() {
    println!(
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
    )
}
