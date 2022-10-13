#!/usr/bin/env -S expect -f

# usage ./test_command.sh <path_of_your_profile> <bucket_name>

set config [lindex $argv 0]
set bucket [lindex $argv 1]
set prompt ")>";  # this my shell prompt, change to yours if you use this script
set timeout 120

spawn dd if=/dev/urandom bs=1024 count=7000 of=/tmp/7M
spawn cargo build
spawn cp README.md test

expect $prompt
spawn target/debug/s3rs --config=$config ls

expect $prompt
spawn target/debug/s3rs --config=$config put test s3://$bucket

expect $prompt
spawn target/debug/s3rs --config=$config ls s3://$bucket

expect $prompt
spawn target/debug/s3rs --config=$config ls /$bucket

expect $prompt
spawn target/debug/s3rs --config=$config ls $bucket

expect $prompt
spawn target/debug/s3rs --config=$config la

expect $prompt
spawn target/debug/s3rs --config=$config ll s3://$bucket/te

expect $prompt
spawn target/debug/s3rs --config=$config cat s3://$bucket/test

expect $prompt
spawn target/debug/s3rs --config=$config tag add s3://$bucket/test a=1 b=2

expect $prompt
spawn target/debug/s3rs --config=$config tag ls s3://$bucket/test

expect $prompt
spawn target/debug/s3rs --config=$config tag del s3://$bucket/test

expect $prompt
spawn target/debug/s3rs --config=$config tag list s3://$bucket/test

expect $prompt
spawn target/debug/s3rs --config=$config rm s3://$bucket/test

expect $prompt
spawn target/debug/s3rs --config=$config ll $bucket

expect $prompt
spawn target/debug/s3rs --config=$config info $bucket

expect $prompt
spawn target/debug/s3rs --config=$config put /tmp/7M s3://$bucket

expect $prompt
spawn target/debug/s3rs --config=$config get s3://$bucket/7M /tmp/7

expect $prompt
spawn md5sum /tmp/7M /tmp/7

interact
spawn rm -f test
