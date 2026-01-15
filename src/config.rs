use crate::error::{AppError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub spotify_client_id: String,
    pub spotify_client_secret: String,
    pub spotify_redirect_uri: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let spotify_client_id = std::env::var("SPOTIFY_CLIENT_ID")
            .map_err(|_| AppError::Config("SPOTIFY_CLIENT_ID not set".into()))?;

        let spotify_client_secret = std::env::var("SPOTIFY_CLIENT_SECRET")
            .map_err(|_| AppError::Config("SPOTIFY_CLIENT_SECRET not set".into()))?;

        let spotify_redirect_uri = std::env::var("SPOTIFY_REDIRECT_URI")
            .unwrap_or_else(|_| "http://127.0.0.1:8080/callback".to_string());

        Ok(Self {
            spotify_client_id,
            spotify_client_secret,
            spotify_redirect_uri,
        })
    }

    pub fn get_missing_config(&self) -> Vec<String> {
        let mut missing = Vec::new();

        if self.spotify_client_id.is_empty() {
            missing.push("SPOTIFY_CLIENT_ID".to_string());
        }
        if self.spotify_client_secret.is_empty() {
            missing.push("SPOTIFY_CLIENT_SECRET".to_string());
        }

        missing
    }

    pub fn validate_spotify_config(&self) -> bool {
        !self.spotify_client_id.is_empty() && !self.spotify_client_secret.is_empty()
    }
}
