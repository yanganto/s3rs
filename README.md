s3rs 
---
a **S3** **R**ich **S**upport client
(There are already a lot of tools, such as s3cmd, I just want to learn rust and radosgw)  
- multi config
- AWS4, AWS2, OSS, GCP support

current status:  

| function                          | descrrption                                 | AWS4 | AWS2 | OSS | GCP |
|-----------------------------------|---------------------------------------------|------|------|-----|-----|
| la                                | list all objects                            | O    | O    | X   | X   |
| ls                                | list bucket                                 | O    | O    | X   | X   |
| ls [bucket]                       | list objects in the bucket                  | O    | O    | X   | X   |
| mb [bucket]                       | create bucket                               | O    | O    | X   | X   |
| rb [bucket]                       | delete bucket                               | O    | O    | X   | X   |
| put [file] s3://[bucket]/[object] | upload the file sepcific object name        | O    | O    | X   | X   |
| put [file] s3://[bucket]          | upload the file use file name as objec name | O    | O    | X   | X   |
| get s3://[bucket]/[object] file   | download objec                              | O    | O    | X   | X   |
| get s3://[bucket]/[object]        | download objec in current folder            | O    | O    | X   | X   |
| /uri?query                        | give the orignal url                        | O    | O    | X   | X   |
|-----------------------------------|---------------------------------------------|------|------|-----|-----|
| s3type [aws/aws4/aws2/ceph/gcp]   | change the api for different S3 providor    | O    | O    | X   | X   |
| log [trace/debug/info/erro]       | change the log level                        | O    | O    | X   | X   |
|                                   | - Debug: for auth signature hash info       |      |      |     |     |
|                                   | - Info: for Http header and body            |      |      |     |     |


# Build Environment
Please download and install Rust and Cargo (Rust package manager)
- [Install Rust](https://www.rust-lang.org/en-US/install.html)
- [Install Cargo](https://crates.io/)

Clone the code
`git clone https://github.com/yanganto/s3rs.git`

# Build
- `cargo build --release`
- The excutable binary will in `./target/release/s3rs`

# Install from cargo
`cargo install s3rs`
