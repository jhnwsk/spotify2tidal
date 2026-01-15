use rspotify::{
    model::{PlaylistId, SimplifiedPlaylist},
    prelude::*,
    scopes, AuthCodeSpotify, ClientCredsSpotify, Credentials, OAuth,
};
use std::io::{self, Write};
use tracing::{debug, info, warn};
use url::Url;

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::spotify::models::{SpotifyPlaylist, SpotifyTrack};

pub struct SpotifyClient {
    client: AuthCodeSpotify,
    user_id: String,
}

impl SpotifyClient {
    pub async fn new(config: &Config) -> Result<Self> {
        let creds = Credentials::new(&config.spotify_client_id, &config.spotify_client_secret);

        let oauth = OAuth {
            redirect_uri: config.spotify_redirect_uri.clone(),
            scopes: scopes!(
                "user-library-read",
                "playlist-read-private",
                "playlist-read-collaborative"
            ),
            ..Default::default()
        };

        let client = AuthCodeSpotify::new(creds, oauth);

        // Get authorization URL
        let auth_url = client.get_authorize_url(false)?;
        println!("\nOpen this URL in your browser to authorize Spotify:");
        println!("{}\n", auth_url);

        print!("Enter the URL you were redirected to: ");
        io::stdout().flush()?;

        let mut redirect_url = String::new();
        io::stdin().read_line(&mut redirect_url)?;

        let code = client
            .parse_response_code(redirect_url.trim())
            .ok_or_else(|| AppError::Auth("Failed to parse authorization code".into()))?;

        client.request_token(&code).await?;

        // Get current user
        let user = client.current_user().await?;
        let user_id = user.id.to_string();
        let display_name = user.display_name.unwrap_or_else(|| user_id.clone());

        info!("Successfully authenticated as Spotify user: {}", display_name);

        Ok(Self { client, user_id })
    }

    pub async fn get_user_playlists(&self) -> Result<Vec<SpotifyPlaylist>> {
        let mut playlists = Vec::new();
        let mut offset = 0;
        let limit = 50;

        loop {
            let page = self
                .client
                .current_user_playlists_manual(Some(limit), Some(offset))
                .await?;

            for playlist in &page.items {
                // Only include playlists owned by the current user
                if playlist.owner.id.to_string() == self.user_id {
                    if let Some(full_playlist) = self.get_playlist_details(playlist).await {
                        playlists.push(full_playlist);
                    }
                }
            }

            if page.next.is_none() {
                break;
            }
            offset += limit;
        }

        info!("Found {} user playlists", playlists.len());
        Ok(playlists)
    }

    async fn get_playlist_details(
        &self,
        playlist: &SimplifiedPlaylist,
    ) -> Option<SpotifyPlaylist> {
        let playlist_id = &playlist.id;
        let name = &playlist.name;
        // SimplifiedPlaylist doesn't have description, we'll leave it empty
        let description = String::new();
        let total_tracks = playlist.tracks.total as usize;
        let public = playlist.public.unwrap_or(false);
        let owner = playlist
            .owner
            .display_name
            .clone()
            .unwrap_or_else(|| playlist.owner.id.to_string());

        info!(
            "Fetching tracks for playlist: {} ({} tracks)",
            name, total_tracks
        );

        match self.get_playlist_tracks(playlist_id).await {
            Ok(tracks) => Some(SpotifyPlaylist {
                id: playlist_id.to_string(),
                name: name.clone(),
                description,
                tracks,
                total_tracks,
                public,
                owner,
            }),
            Err(e) => {
                warn!("Failed to get playlist details for {}: {}", name, e);
                None
            }
        }
    }

    async fn get_playlist_tracks(&self, playlist_id: &PlaylistId<'_>) -> Result<Vec<SpotifyTrack>> {
        let mut tracks = Vec::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            let page = self
                .client
                .playlist_items_manual(playlist_id.clone_static(), None, None, Some(limit), Some(offset))
                .await?;

            for item in &page.items {
                if let Some(rspotify::model::PlayableItem::Track(track)) = &item.track {
                    // Skip local tracks (they don't have an ID)
                    if track.id.is_none() {
                        debug!("Skipping local track: {}", track.name);
                        continue;
                    }

                    let spotify_track = SpotifyTrack {
                        id: track.id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
                        name: track.name.clone(),
                        artists: track.artists.iter().map(|a| a.name.clone()).collect(),
                        album: track.album.name.clone(),
                        duration_ms: track.duration.num_milliseconds() as u64,
                        isrc: track
                            .external_ids
                            .get("isrc")
                            .cloned(),
                        popularity: track.popularity as u8,
                    };
                    tracks.push(spotify_track);
                }
            }

            if page.next.is_none() {
                break;
            }
            offset += limit;
        }

        Ok(tracks)
    }

    pub async fn get_playlist_by_name(&self, name: &str) -> Result<Option<SpotifyPlaylist>> {
        let playlists = self.get_user_playlists().await?;
        Ok(playlists
            .into_iter()
            .find(|p| p.name.to_lowercase() == name.to_lowercase()))
    }
}

/// Client for fetching public Spotify playlists using client credentials.
/// Does not require user authentication - only app credentials.
pub struct PublicSpotifyClient {
    client: ClientCredsSpotify,
}

impl PublicSpotifyClient {
    pub async fn new(config: &Config) -> Result<Self> {
        let creds = Credentials::new(&config.spotify_client_id, &config.spotify_client_secret);
        let client = ClientCredsSpotify::new(creds);

        client.request_token().await?;
        info!("Authenticated with Spotify using client credentials");

        Ok(Self { client })
    }

    /// Parse a Spotify playlist URL and extract the playlist ID.
    /// Supports formats:
    /// - https://open.spotify.com/playlist/37i9dQZF1E8NC99vGqLsaH
    /// - https://open.spotify.com/playlist/37i9dQZF1E8NC99vGqLsaH?si=...
    /// - spotify:playlist:37i9dQZF1E8NC99vGqLsaH
    pub fn parse_playlist_url(url_str: &str) -> Result<String> {
        // Handle Spotify URI format
        if url_str.starts_with("spotify:playlist:") {
            return Ok(url_str.replace("spotify:playlist:", ""));
        }

        // Handle URL format
        let url = Url::parse(url_str)
            .map_err(|e| AppError::Config(format!("Invalid URL: {}", e)))?;

        let path_segments: Vec<&str> = url.path_segments()
            .ok_or_else(|| AppError::Config("Invalid Spotify URL".into()))?
            .collect();

        // Expect /playlist/{id}
        if path_segments.len() >= 2 && path_segments[0] == "playlist" {
            Ok(path_segments[1].to_string())
        } else {
            Err(AppError::Config(
                "URL does not appear to be a Spotify playlist URL".into(),
            ))
        }
    }

    /// Fetch a public playlist by its ID.
    pub async fn get_playlist(&self, playlist_id: &str) -> Result<SpotifyPlaylist> {
        let id = PlaylistId::from_id(playlist_id)
            .map_err(|e| AppError::Config(format!("Invalid playlist ID: {}", e)))?;

        let playlist = self.client.playlist(id.clone_static(), None, None).await?;

        info!(
            "Fetching public playlist: {} ({} tracks)",
            playlist.name,
            playlist.tracks.total
        );

        let tracks = self.get_playlist_tracks(&id).await?;

        Ok(SpotifyPlaylist {
            id: playlist_id.to_string(),
            name: playlist.name,
            description: playlist.description.unwrap_or_default(),
            tracks,
            total_tracks: playlist.tracks.total as usize,
            public: playlist.public.unwrap_or(true),
            owner: playlist
                .owner
                .display_name
                .unwrap_or_else(|| playlist.owner.id.to_string()),
        })
    }

    async fn get_playlist_tracks(&self, playlist_id: &PlaylistId<'_>) -> Result<Vec<SpotifyTrack>> {
        let mut tracks = Vec::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            let page = self
                .client
                .playlist_items_manual(playlist_id.clone_static(), None, None, Some(limit), Some(offset))
                .await?;

            for item in &page.items {
                if let Some(rspotify::model::PlayableItem::Track(track)) = &item.track {
                    // Skip local tracks (they don't have an ID)
                    if track.id.is_none() {
                        debug!("Skipping local track: {}", track.name);
                        continue;
                    }

                    let spotify_track = SpotifyTrack {
                        id: track.id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
                        name: track.name.clone(),
                        artists: track.artists.iter().map(|a| a.name.clone()).collect(),
                        album: track.album.name.clone(),
                        duration_ms: track.duration.num_milliseconds() as u64,
                        isrc: track.external_ids.get("isrc").cloned(),
                        popularity: track.popularity as u8,
                    };
                    tracks.push(spotify_track);
                }
            }

            if page.next.is_none() {
                break;
            }
            offset += limit;
        }

        info!("Fetched {} tracks from playlist", tracks.len());
        Ok(tracks)
    }
}
