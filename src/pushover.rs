use serde::Deserialize;

use crate::request::client_with_retry;

#[derive(Debug, Deserialize)]
pub struct Pushover {
    pub token: String,
    pub user: String,
}

impl Pushover {
    pub async fn send(
        &self,
        message: &str,
        title: Option<&str>,
    ) -> Result<(), String> {
        let mut form_data = vec![
            ("token", self.token.as_str()),
            ("user", self.user.as_str()),
            ("message", message),
        ];

        if let Some(x) = title {
            form_data.push(("title", x));
        }

        let status_code = client_with_retry()
            .post("https://api.pushover.net/1/messages.json")
            .form(&form_data)
            .send()
            .await
            .map_err(|x| x.to_string())?
            .status()
            .as_u16();

        if status_code == 200 {
            Ok(())
        } else {
            Err(format!("pushover: status code {status_code}"))
        }
    }
}
