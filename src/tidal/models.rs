use serde::{Deserialize, Serialize};

use crate::spotify::models::SpotifyTrack;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TidalTrack {
    pub id: u64,
    pub name: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_secs: u64,
    pub isrc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TidalPlaylist {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tracks: Vec<TidalTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub spotify_track: SpotifyTrack,
    pub tidal_track: Option<TidalTrack>,
    pub match_score: f64,
    pub match_method: MatchMethod,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchMethod {
    Isrc,
    Exact,
    Fuzzy,
    NoMatch,
}

impl std::fmt::Display for MatchMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchMethod::Isrc => write!(f, "ISRC"),
            MatchMethod::Exact => write!(f, "Exact"),
            MatchMethod::Fuzzy => write!(f, "Fuzzy"),
            MatchMethod::NoMatch => write!(f, "NoMatch"),
        }
    }
}

#[cfg(test)]
impl TidalTrack {
    pub fn mock(name: &str, artist: &str) -> Self {
        Self {
            id: 12345,
            name: name.to_string(),
            artists: vec![artist.to_string()],
            album: "Mock Album".to_string(),
            duration_secs: 180,
            isrc: Some("MOCK12345678".to_string()),
        }
    }
}
