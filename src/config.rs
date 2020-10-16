use crate::command::secret::decrypt_config;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub credential: Option<Vec<s3handler::CredentialConfig>>,
}
impl<'a> Config {
    pub fn gen_selections(&'a self) -> Vec<String> {
        let mut display_list = Vec::new();
        let credential = &self.credential.clone().unwrap();
        for cre in credential.into_iter() {
            let c = cre.clone();
            let option = String::from(format!(
                "[{}] {} ({}) {} ({})",
                c.s3_type.unwrap_or(String::from("aws")),
                c.host,
                c.region.unwrap_or(String::from("us-east-1")),
                c.user.unwrap_or(String::from("user")),
                c.access_key
            ));
            display_list.push(option);
        }
        display_list
    }

    pub fn decrypt(&'a mut self, run_time_secret: &Vec<u8>) {
        if run_time_secret.len() > 0 {
            for cre in self.credential.iter_mut() {
                for c in cre.iter_mut() {
                    decrypt_config(run_time_secret, c);
                }
            }
        }
    }
}
