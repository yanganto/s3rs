s3rs 
---
[![Build Status](https://travis-ci.com/yanganto/s3rs.svg?branch=master)](https://travis-ci.com/yanganto/s3rs)  

an **S3** **R**ich **S**upport client
- rust edition 2018
- multi config
- interactive command line tool
- easy to debug with http protocol
- AWS4, AWS2 support
- support http redirect for multi region of AWS S3
- support automatically multipart upload

current status:  

| function                                                  | descrrption                                 | CEPH | AWS |
|-----------------------------------------------------------|---------------------------------------------|------|-----|
| la                                                        | list all objects                            | O    | O   |
| ls                                                        | list bucket                                 | O    | O   |
| ls [bucket]                                               | list objects in the bucket                  | O    | O   |
| ls s3://[bucket]                                          | list objects in the bucket                  | O    | O   |
| mb [bucket]                                               | create bucket                               | O    | O   |
| rb [bucket]                                               | delete bucket                               | O    | O   |
| put [file] s3://[bucket]/[object]                         | upload the file sepcific object name        | O    | O   |
| put [file] s3://[bucket]                                  | upload the file use file name as objec name | O    | O   |
| put test s3://[bucket]/[object]                           | upload a test file sepcific object name     | O    | O   |
| get s3://[bucket]/[object] file                           | download objec                              | O    | O   |
| get s3://[bucket]/[object]                                | download objec in current folder            | O    | O   |
| cat s3://[bucket]/[object]                                | show the object content                     | O    | O   |
| del s3://[bucket]/[object]                                | delete the object                           | O    | O   |
| tag list s3://[bucket]/[object] key1=value1 [key2=value2] | list tag(s) to the object                   | O    | O   |
| tag ls s3://[bucket]/[object] key1=value1 [key2=value2]   | list tag(s) to the object                   | O    | O   |
| tag add s3://[bucket]/[object] key1=value1 [key2=value2]  | add tag(s) to the object                    | O    | O   |
| tag put s3://[bucket]/[object] key1=value1 [key2=value2]  | add tag(s) to the object                    | O    | O   |
| tag del s3://[bucket]/[object]                            | remove tag(s) from the object               | O    | O   |
| tag rm s3://[bucket]/[object]                             | remove tag(s) from the object               | O    | O   |
| usage s3://[bucket]                                       | show the bucket usage (ceph only)           | O    | -   |
| /uri?query                                                | give the orignal url                        | O    | O   |
|-----------------------------------------------------------|---------------------------------------------|------|-----|
| s3\_type [ceph/aws/aws4/aws2]                             | change the api for different S3 providor    |      |     |
| log [trace/debug/info/erro]                               | change the log level                        |      |     |
|                                                           | - trace: more detail about rust             |      |     |
|                                                           | - debug: for auth signature hash info       |      |     |
|                                                           | - Info: for Http header and body            |      |     |
| logout                                                    | logout and reselect user                    |      |     |
| Ctrl + d                                                  | logout and reselect user                    |      |     |


| s3 type | auth type | format | virtual-hosted–style path-style |
|---------|-----------|--------|---------------------------------|
| ceph    | aws4      | json   | path-style                      |
| aws     | aws4      | xml    | virtual-hosted–style            |

# Install 
Install rust tools rustup and cargo 
- `curl https://sh.rustup.rs -sSf | sh`

Additional package for ubuntu
- `apt-get install libssl-dev`

It is easy to install s3rs via cargo as following command.
- `cargo install s3rs`

Set up the path for rust binnary
- `export PATH=$PATH:~/.cargo/bin`

The config file will auto setup when binary first time launch
- `s3rs`

edit the config file at ~/.s3rs.toml
- `vi ~/.s3rs.toml`


# Develop
Install rust tools rustup and cargo 
- `curl https://sh.rustup.rs -sSf | sh`
- `git clone https://github.com/yanganto/s3rs.git`
- `cargo test`
- `cargo build`
- The excutable binary will in `./target/debug/s3rs`

# Demo
- A short demo [video](https://youtu.be/MtPYhJnbMfs)
- snapshot
[![snapshot](https://raw.githubusercontent.com/yanganto/s3rs/master/example.png)](https://youtu.be/MtPYhJnbMfs)
[![snapshot](https://raw.githubusercontent.com/yanganto/s3rs/master/example2.png)](https://youtu.be/MtPYhJnbMfs)

