use std::error::Error;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};

use crate::identity::{AccessResponse, IdentityStore, Outcome};
use serde_derive::Deserialize;

use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct JsonIdentitySettings {
    pub filename: String,
}

#[derive(Deserialize)]
struct User {
    username: String,
    token: String,
    access: bool,
    valid_time: DateTime<Utc>,
}

pub struct Json {
    settings: JsonIdentitySettings,
}

impl Json {
    pub fn new(settings: JsonIdentitySettings) -> Self {
        Self { settings }
    }
}

#[async_trait]
impl IdentityStore for Json {
    async fn access(&mut self, token: &str) -> Result<AccessResponse, Box<dyn Error>> {
        let reader = BufReader::new(
            OpenOptions::new()
                .read(true)
                .open(&self.settings.filename)?,
        );

        for (n, line) in reader.lines().enumerate() {
            let location = format!("{}:{}", self.settings.filename, n + 1);
            match line {
                Err(e) => {
                    warn!("Failed to read user at {}: {:?}", location, e);
                }
                Ok(line) => match serde_json::from_str::<User>(&line) {
                    Err(e) => {
                        warn!("Failed to deserialize user at {}: {:?}", location, e);
                    }
                    Ok(user) => {
                        if user.token == token && !is_expired(user.valid_time) {
                            return Ok(AccessResponse {
                                outcome: if user.access {
                                    Outcome::Success
                                } else {
                                    Outcome::Revoked
                                },
                                name: Some(user.username),
                            });
                        }
                    }
                },
            }
        }

        return Ok(AccessResponse {
            outcome: Outcome::Unknown,
            name: None,
        });
    }
}

fn is_expired(expiry: DateTime<Utc>) -> bool {
    Utc::now() > expiry
}
