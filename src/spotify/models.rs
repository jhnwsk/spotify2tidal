use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTrack {
    pub id: String,
    pub name: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_ms: u64,
    pub isrc: Option<String>,
    pub popularity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tracks: Vec<SpotifyTrack>,
    pub total_tracks: usize,
    pub public: bool,
    pub owner: String,
}

#[cfg(test)]
impl SpotifyTrack {
    pub fn mock(name: &str, artist: &str) -> Self {
        Self {
            id: "mock_id".to_string(),
            name: name.to_string(),
            artists: vec![artist.to_string()],
            album: "Mock Album".to_string(),
            duration_ms: 180000,
            isrc: Some("MOCK12345678".to_string()),
            popularity: 50,
        }
    }
}
