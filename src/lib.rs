pub mod config;
pub mod error;
pub mod matcher;
pub mod migrator;
pub mod spotify;
pub mod tidal;

pub use config::Config;
pub use error::{AppError, Result};
pub use migrator::PlaylistMigrator;
pub use spotify::{SpotifyClient, SpotifyPlaylist, SpotifyTrack};
pub use tidal::{MatchMethod, MatchResult, TidalClient, TidalPlaylist, TidalTrack};
