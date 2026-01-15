use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub playlist_name: String,
    pub total_tracks: usize,
    pub successful_matches: usize,
    pub failed_matches: usize,
    pub success_rate: f64,
    pub failed_tracks: Vec<FailedTrack>,
    pub tidal_playlist_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedTrack {
    pub name: String,
    pub artist: String,
    pub album: String,
}

impl MigrationResult {
    pub fn new(playlist_name: String, total_tracks: usize) -> Self {
        Self {
            playlist_name,
            total_tracks,
            successful_matches: 0,
            failed_matches: 0,
            success_rate: 0.0,
            failed_tracks: Vec::new(),
            tidal_playlist_id: None,
        }
    }

    pub fn calculate_success_rate(&mut self) {
        if self.total_tracks > 0 {
            self.success_rate = (self.successful_matches as f64 / self.total_tracks as f64) * 100.0;
        }
    }
}
