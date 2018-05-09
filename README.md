s3rs 
---
(I just want to learn rust and rados)  
a **S3** **R**ich **S**upport client
- multi config
- AWS4, AWS2, OSS support

current status:  

| function                    | descrrption                              | AWS4 | AWS2 | OSS |
|-----------------------------|------------------------------------------|------|------|-----|
| la                          | list objects                             | O    | O    | X   |
| ls                          | list bucket                              | O    | O    | X   |
| mb [bucket]                 | create bucket                            | O    | O    | X   |
| rb [bucket]                 | delete bucket                            | O    | O    | X   |
| /uri?query                  | give the orignal url                     | O    | O    | X   |
| s3type [aws/aws4/aws2/ceph] | change the api for different S3 providor | O    | O    | X   |
| log [trace/debug/info/erro] | change the log level                     | O    | O    | X   |
