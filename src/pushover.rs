use anyhow::{anyhow, Result};
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::request;

#[derive(Debug, Deserialize, Clone)]
pub struct Pushover {
    pub token: String,
    pub user: String,
}

impl Pushover {
    pub async fn send(
        &self,
        message: &str,
        title: Option<&str>,
        cancellation_token: CancellationToken,
    ) -> Result<()> {
        let mut form_data = vec![
            ("token", self.token.as_str()),
            ("user", self.user.as_str()),
            ("message", message),
        ];

        if let Some(x) = title {
            form_data.push(("title", x));
        }

        let status_code = request::post(
            "https://api.pushover.net/1/messages.json",
            &form_data,
            cancellation_token,
        )
        .await?
        .status()
        .as_u16();

        if status_code == 200 {
            Ok(())
        } else {
            Err(anyhow!("pushover: status code {status_code}"))
        }
    }
}
