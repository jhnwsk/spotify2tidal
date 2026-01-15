use chrono::Local;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;
use tracing::{info, warn};

use crate::config::Config;
use crate::error::Result;
use crate::migrator::report::{FailedTrack, MigrationResult};
use crate::spotify::{SpotifyClient, SpotifyPlaylist, SpotifyTrack};
use crate::tidal::{TidalClient, TidalTrack};

pub struct PlaylistMigrator {
    spotify_client: SpotifyClient,
    tidal_client: TidalClient,
}

impl PlaylistMigrator {
    pub async fn new(
        config: &Config,
        tidal_client_id: &str,
        tidal_client_secret: &str,
    ) -> Result<Self> {
        let spotify_client = SpotifyClient::new(config).await?;
        let tidal_client = TidalClient::new(tidal_client_id, tidal_client_secret).await?;

        Ok(Self {
            spotify_client,
            tidal_client,
        })
    }

    pub async fn migrate_all_playlists(&self, dry_run: bool) -> Result<Vec<MigrationResult>> {
        let playlists = self.spotify_client.get_user_playlists().await?;
        let mut results = Vec::new();

        info!(
            "Starting migration of {} playlists (dry_run={})",
            playlists.len(),
            dry_run
        );

        let pb = ProgressBar::new(playlists.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        for playlist in &playlists {
            pb.set_message(format!("Migrating: {}", playlist.name));
            let result = self.migrate_playlist(playlist, dry_run).await;
            results.push(result);
            pb.inc(1);
        }

        pb.finish_with_message("Migration complete");

        self.save_migration_results(&results)?;
        self.print_summary(&results);

        Ok(results)
    }

    pub async fn migrate_specific_playlists(
        &self,
        playlist_names: &[String],
        dry_run: bool,
    ) -> Result<Vec<MigrationResult>> {
        let all_playlists = self.spotify_client.get_user_playlists().await?;

        let target_playlists: Vec<_> = playlist_names
            .iter()
            .filter_map(|name| {
                all_playlists
                    .iter()
                    .find(|p| p.name.to_lowercase() == name.to_lowercase())
            })
            .collect();

        if target_playlists.is_empty() {
            warn!("No valid playlists found to migrate");
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        for playlist in target_playlists {
            let result = self.migrate_playlist(playlist, dry_run).await;
            results.push(result);
        }

        self.save_migration_results(&results)?;
        self.print_summary(&results);

        Ok(results)
    }

    async fn migrate_playlist(
        &self,
        spotify_playlist: &SpotifyPlaylist,
        dry_run: bool,
    ) -> MigrationResult {
        info!("Migrating playlist: {}", spotify_playlist.name);

        let mut result = MigrationResult::new(
            spotify_playlist.name.clone(),
            spotify_playlist.tracks.len(),
        );

        let mut matched_tracks: Vec<(SpotifyTrack, TidalTrack)> = Vec::new();

        let pb = ProgressBar::new(spotify_playlist.tracks.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {spinner:.green} [{bar:30.cyan/blue}] {pos}/{len}")
                .unwrap()
                .progress_chars("#>-"),
        );

        for track in &spotify_playlist.tracks {
            match self.tidal_client.search_track(track).await {
                Some(tidal_track) => {
                    result.successful_matches += 1;
                    matched_tracks.push((track.clone(), tidal_track));
                }
                None => {
                    result.failed_matches += 1;
                    result.failed_tracks.push(FailedTrack {
                        name: track.name.clone(),
                        artist: track.artists.join(", "),
                        album: track.album.clone(),
                    });
                }
            }
            pb.inc(1);
        }

        pb.finish_and_clear();
        result.calculate_success_rate();

        // Create Tidal playlist if not dry run and we have matches
        if !dry_run && !matched_tracks.is_empty() {
            result.tidal_playlist_id = self
                .create_tidal_playlist(spotify_playlist, &matched_tracks)
                .await;
        }

        info!(
            "Playlist migration completed: {} - {}/{} tracks matched ({:.1}% success rate)",
            spotify_playlist.name,
            result.successful_matches,
            result.total_tracks,
            result.success_rate
        );

        result
    }

    async fn create_tidal_playlist(
        &self,
        spotify_playlist: &SpotifyPlaylist,
        matched_tracks: &[(SpotifyTrack, TidalTrack)],
    ) -> Option<String> {
        let description = format!(
            "Migrated from Spotify. {}",
            spotify_playlist.description
        );

        match self
            .tidal_client
            .create_playlist(&spotify_playlist.name, &description)
            .await
        {
            Ok(playlist) => {
                let track_ids: Vec<u64> = matched_tracks.iter().map(|(_, t)| t.id).collect();

                // Add tracks in batches of 100
                let batch_size = 100;
                for (i, chunk) in track_ids.chunks(batch_size).enumerate() {
                    match self
                        .tidal_client
                        .add_tracks_to_playlist(&playlist.id, chunk)
                        .await
                    {
                        Ok(true) => {}
                        Ok(false) | Err(_) => {
                            warn!(
                                "Failed to add batch {} to playlist {}",
                                i + 1,
                                spotify_playlist.name
                            );
                        }
                    }
                }

                Some(playlist.id)
            }
            Err(e) => {
                warn!(
                    "Failed to create Tidal playlist for {}: {}",
                    spotify_playlist.name, e
                );
                None
            }
        }
    }

    fn save_migration_results(&self, results: &[MigrationResult]) -> Result<()> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let results_dir = Path::new("migration_results");

        fs::create_dir_all(results_dir)?;

        let filename = results_dir.join(format!("migration_results_{}.json", timestamp));
        let json = serde_json::to_string_pretty(results)?;

        fs::write(&filename, json)?;

        info!("Migration results saved to: {}", filename.display());

        Ok(())
    }

    fn print_summary(&self, results: &[MigrationResult]) {
        let total_playlists = results.len();
        let total_tracks: usize = results.iter().map(|r| r.total_tracks).sum();
        let total_successful: usize = results.iter().map(|r| r.successful_matches).sum();
        let total_failed: usize = results.iter().map(|r| r.failed_matches).sum();

        let overall_success_rate = if total_tracks > 0 {
            (total_successful as f64 / total_tracks as f64) * 100.0
        } else {
            0.0
        };

        println!();
        println!("{}", "=".repeat(60));
        println!("{}", "MIGRATION SUMMARY".bold());
        println!("{}", "=".repeat(60));
        println!("Total playlists processed: {}", total_playlists);
        println!("Total tracks processed: {}", total_tracks);
        println!(
            "Successfully matched: {}",
            total_successful.to_string().green()
        );
        println!("Failed to match: {}", total_failed.to_string().red());
        println!(
            "Overall success rate: {:.1}%",
            overall_success_rate
        );
        println!("{}", "=".repeat(60));

        println!("\nPlaylist breakdown:");
        for result in results {
            let status = if result.success_rate >= 90.0 {
                format!("{:.1}%", result.success_rate).green()
            } else if result.success_rate >= 70.0 {
                format!("{:.1}%", result.success_rate).yellow()
            } else {
                format!("{:.1}%", result.success_rate).red()
            };

            println!(
                "  {}: {}/{} ({})",
                result.playlist_name,
                result.successful_matches,
                result.total_tracks,
                status
            );
        }

        if results.iter().any(|r| !r.failed_tracks.is_empty()) {
            println!(
                "\n{}",
                "Failed tracks have been logged and saved to migration_results/".yellow()
            );
        }
    }
}
