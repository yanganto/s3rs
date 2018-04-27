extern crate toml;

#[macro_use]
extern crate serde_derive;

use std::io;
use std::io::{Read, Write, BufReader, BufRead};
use std::fs::{File, OpenOptions};
use std::str::FromStr;

#[derive(Debug, Deserialize)]
struct CredentialConfig {
    host: String,
    user: Option<String>,
    access_key: String,
    secrete_key: String
}

#[derive(Debug, Deserialize)]
struct Config {
    credential: Option<Vec<CredentialConfig>>
}
impl Config {
    fn gen_selecitons_and_values(self) -> (Vec<String>, Vec<(String, String)>) {
        let mut display_list = Vec::new();
        let mut value_list = Vec::new();
        for c in self.credential.unwrap().into_iter(){
            let mut option = String::from(c.host);
            option.push('|');
            option.push_str(c.user.unwrap_or(String::from("")).as_str());
            option.push('|');
            option.push_str(c.access_key.as_str());
            display_list.push(option);
            value_list.push((c.access_key, c.secrete_key));
        }
        (display_list, value_list)
        // self.credential.unwrap().into_iter().map(|x|x.access_key).collect()
    }
}


fn read_parse<T>(tty: &mut File, prompt: &str, min: T, max: T) -> io::Result<T> where T: FromStr + Ord {
    try!(tty.write_all(prompt.as_bytes()));
    let mut reader = BufReader::new(tty);
    let mut result = String::new();
    try!(reader.read_line(&mut result));
    match result.replace("\n", "").parse::<T>() {
        Ok(x) => if x >= min && x <= max { Ok(x) } else { read_parse(reader.into_inner(), prompt, min, max) },
        _ => read_parse(reader.into_inner(), prompt, min, max)
    }
}

fn my_pick_from_list_internal<T: AsRef<str>>(items: &[T], prompt: &str) -> io::Result<usize> {
    let mut tty = try!(OpenOptions::new().read(true).write(true).open("/dev/tty"));
    let pad_len = ((items.len() as f32).log10().floor() + 1.0) as usize;
    for (i, item) in items.iter().enumerate() {
        try!(tty.write_all(format!("{1:0$}. {2}\n", pad_len, i + 1, item.as_ref().replace("\n", "")).as_bytes()))
    }
    let idx = try!(read_parse::<usize>(&mut tty, prompt, 1, items.len())) - 1;
    Ok(idx)
}

fn main() {

    let mut s3rscfg = std::env::home_dir().unwrap();
    s3rscfg.push(".s3rs");

    let mut f = File::open(s3rscfg).expect("s3rs config file not found");

    let mut config_contents = String::new();
    f.read_to_string(&mut config_contents).expect("s3rs config is not readable");

    let config:Config = toml::from_str(config_contents.as_str()).unwrap();


    let config_option_and_value: (Vec<String>, Vec<(String, String)>) = config.gen_selecitons_and_values();

    // println!("{:?}", config_option_and_value.0);
    let chosen_int = my_pick_from_list_internal(&config_option_and_value.0, "Selection: ").unwrap();

    println!("credential chosen, you chose '{}'!!", config_option_and_value.1[chosen_int].1);
    // println!("{:?}", );

    //println!("credential chosen, you chose '{:?}'!!", config.credential.unwrap()[0].user);

}
