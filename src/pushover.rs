use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Pushover {
    pub token: String,
    pub user: String,
}
