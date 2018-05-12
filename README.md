s3rs 
---
(I just want to learn rust and rados)  
a **S3** **R**ich **S**upport client
- multi config
- AWS4, AWS2, OSS, GCP support

current status:  

| function                         | descrrption                                 | AWS4 | AWS2 | OSS | GCP |
|----------------------------------|---------------------------------------------|------|------|-----|-----|
| la                               | list all objects                            | O    | O    | X   | X   |
| ls                               | list bucket                                 | O    | O    | X   | X   |
| ls [bucket]                      | list objects in the bucket                  | O    | O    | X   | X   |
| mb [bucket]                      | create bucket                               | O    | O    | X   | X   |
| rb [bucket]                      | delete bucket                               | O    | O    | X   | X   |
| put [file] s3://[bucket][object] | upload the file sepcific object name        | O    | O    | X   | X   |
| put [file] s3://[bucket]         | upload the file use file name as objec name | O    | O    | X   | X   |
| get s3://[bucket][object] file   | download objec                              | O    | O    | X   | X   |
| get s3://[bucket][object]        | download objec in current folder            | O    | O    | X   | X   |
| /uri?query                       | give the orignal url                        | O    | O    | X   | X   |
|----------------------------------|---------------------------------------------|------|------|-----|-----|
| s3type [aws/aws4/aws2/ceph/gcp]  | change the api for different S3 providor    | O    | O    | X   | X   |
| log [trace/debug/info/erro]      | change the log level                        | O    | O    | X   | X   |
