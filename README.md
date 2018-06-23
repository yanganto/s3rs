**This project is moved to gitlab** [gitlab repo link](https://gitlab.com/yanganto/s3rs)

s3rs 
---
a **S3** **R**ich **S**upport client
(There are already a lot of tools, such as s3cmd, I just want to learn rust and radosgw)  
- multi config
- AWS4, AWS2 support
- TODO: OSS, GCP support

current status:  

| function                          | descrrption                                 | AWS4 | AWS2 |
|-----------------------------------|---------------------------------------------|------|------|
| la                                | list all objects                            | O    | O    |
| ls                                | list bucket                                 | O    | O    |
| ls [bucket]                       | list objects in the bucket                  | O    | O    |
| ls s3://[bucket]                  | list objects in the bucket                  | O    | O    |
| mb [bucket]                       | create bucket                               | O    | O    |
| rb [bucket]                       | delete bucket                               | O    | O    |
| put [file] s3://[bucket]/[object] | upload the file sepcific object name        | O    | O    |
| put [file] s3://[bucket]          | upload the file use file name as objec name | O    | O    |
| put test s3://[bucket]/[object]   | upload a test file sepcific object name     | O    | O    |
| get s3://[bucket]/[object] file   | download objec                              | O    | O    |
| get s3://[bucket]/[object]        | download objec in current folder            | O    | O    |
| cat s3://[bucket]/[object]        | show the object content                     | O    | O    |
| del s3://[bucket]/[object]        | delete the object                           | O    | O    |
| /uri?query                        | give the orignal url                        | O    | O    |
|-----------------------------------|---------------------------------------------|------|------|
| s3type [aws/aws4/aws2/ceph/gcp]   | change the api for different S3 providor    | O    | O    |
| log [trace/debug/info/erro]       | change the log level                        | O    | O    |
|                                   | - Debug: for auth signature hash info       |      |      |
|                                   | - Info: for Http header and body            |      |      |


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

# Demo
- A short demo [video](https://youtu.be/DnWQbDmBFpg)
