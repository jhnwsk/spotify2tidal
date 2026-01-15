use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Spotify API error: {0}")]
    SpotifyApi(#[from] rspotify::ClientError),

    #[error("Tidal API error: {0}")]
    TidalApi(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Playlist not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
