#!/usr/bin/expect -f

# usage ./test_command.sh <number_of_your_profile> <bucket_name>

set item [lindex $argv 0]
set bucket [lindex $argv 1]
set prompt "s3rs.*>";

spawn cargo run

expect "Selection:"
send $item\r

expect -re $prompt
send ls\r

expect -re $prompt
send "put test s3://$bucket\r"

expect -re $prompt
send "ls s3://$bucket\r"

expect -re $prompt
send "ls /$bucket\r"

expect -re $prompt
send "ls $bucket\r"

expect -re $prompt
send "la\r"

expect -re $prompt
send "ll s3://$bucket/te\r"

expect -re $prompt
send "cat s3://$bucket/test\r"

expect -re $prompt
send "tag add s3://$bucket/test a=1 b=2\r"

expect -re $prompt
send "tag ls s3://$bucket/test\r"

expect -re $prompt
send "tag del s3://$bucket/test\r"

expect -re $prompt
send "tag list s3://$bucket/test\r"

expect -re $prompt
send "rm s3://$bucket/test\r"

expect -re $prompt
send "ll $bucket\r"

expect -re $prompt
send "info $bucket\r"

expect -re $prompt
send "logout\n"

expect "Selection:"
send $item\r

expect -re $prompt
send "exit\n"

interact

