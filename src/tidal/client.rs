use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use tracing::{debug, info, warn};

use crate::error::{AppError, Result};
use crate::matcher::{calculate_similarity, is_exact_match, is_fuzzy_match};
use crate::spotify::SpotifyTrack;
use crate::tidal::models::{TidalPlaylist, TidalTrack};

const TIDAL_API_BASE: &str = "https://openapi.tidal.com/v2";
const TIDAL_AUTH_URL: &str = "https://auth.tidal.com/v1/oauth2";

#[derive(Debug, Deserialize)]
struct DeviceAuthResponse {
    #[serde(rename = "deviceCode")]
    device_code: String,
    #[serde(rename = "userCode")]
    user_code: String,
    #[serde(rename = "verificationUri")]
    verification_uri: String,
    #[serde(rename = "verificationUriComplete")]
    verification_uri_complete: Option<String>,
    #[serde(rename = "expiresIn")]
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct TidalSearchResponse {
    tracks: Option<Vec<TidalApiTrack>>,
}

#[derive(Debug, Deserialize)]
struct TidalApiTrack {
    id: u64,
    title: String,
    artists: Vec<TidalApiArtist>,
    album: Option<TidalApiAlbum>,
    duration: u64,
    isrc: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TidalApiArtist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct TidalApiAlbum {
    title: String,
}

#[derive(Debug, Serialize)]
struct CreatePlaylistRequest {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TidalApiPlaylist {
    uuid: String,
    name: String,
    description: Option<String>,
}

pub struct TidalClient {
    http_client: Client,
    access_token: String,
    user_id: String,
}

impl TidalClient {
    pub async fn new(client_id: &str, client_secret: &str) -> Result<Self> {
        let http_client = Client::new();

        // Device authorization flow
        let device_auth = Self::device_authorization(&http_client, client_id).await?;

        println!("\nTidal Authentication Required");
        println!("==============================");
        if let Some(uri) = &device_auth.verification_uri_complete {
            println!("Visit this URL: {}", uri);
        } else {
            println!("Visit: {}", device_auth.verification_uri);
            println!("Enter code: {}", device_auth.user_code);
        }
        println!("\nWaiting for authentication...");

        // Poll for token
        let token = Self::poll_for_token(
            &http_client,
            client_id,
            client_secret,
            &device_auth.device_code,
            device_auth.interval,
            device_auth.expires_in,
        )
        .await?;

        info!("Successfully authenticated with Tidal");

        // Get user ID (simplified - in real implementation would call user endpoint)
        let user_id = "me".to_string();

        Ok(Self {
            http_client,
            access_token: token.access_token,
            user_id,
        })
    }

    async fn device_authorization(
        client: &Client,
        client_id: &str,
    ) -> Result<DeviceAuthResponse> {
        let response = client
            .post(format!("{}/device_authorization", TIDAL_AUTH_URL))
            .form(&[
                ("client_id", client_id),
                ("scope", "playlists.read playlists.write"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Auth(format!(
                "Device authorization failed: {}",
                error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Auth(format!("Failed to parse device auth response: {}", e)))
    }

    async fn poll_for_token(
        client: &Client,
        client_id: &str,
        client_secret: &str,
        device_code: &str,
        interval: u64,
        expires_in: u64,
    ) -> Result<TokenResponse> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(expires_in);

        loop {
            if start.elapsed() > timeout {
                return Err(AppError::Auth("Device authorization timed out".into()));
            }

            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

            let response = client
                .post(format!("{}/token", TIDAL_AUTH_URL))
                .basic_auth(client_id, Some(client_secret))
                .form(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", device_code),
                ])
                .send()
                .await?;

            if response.status().is_success() {
                return response.json().await.map_err(|e| {
                    AppError::Auth(format!("Failed to parse token response: {}", e))
                });
            }

            // Check if still pending
            let error_text = response.text().await.unwrap_or_default();
            if !error_text.contains("authorization_pending") {
                return Err(AppError::Auth(format!("Token request failed: {}", error_text)));
            }

            print!(".");
            io::stdout().flush().ok();
        }
    }

    pub async fn search_track(&self, track: &SpotifyTrack) -> Option<TidalTrack> {
        // Tier 1: Search by ISRC
        if let Some(isrc) = &track.isrc {
            if let Some(found) = self.search_by_isrc(isrc).await {
                debug!("Found track by ISRC: {}", track.name);
                return Some(found);
            }
        }

        // Tier 2: Exact match search
        if let Some(found) = self.search_exact(track).await {
            debug!("Found track by exact match: {}", track.name);
            return Some(found);
        }

        // Tier 3: Fuzzy match search
        if let Some(found) = self.search_fuzzy(track).await {
            debug!("Found track by fuzzy match: {}", track.name);
            return Some(found);
        }

        debug!("No match found for track: {}", track.name);
        None
    }

    async fn search_by_isrc(&self, isrc: &str) -> Option<TidalTrack> {
        let results = self.search_tracks(isrc).await.ok()?;

        for track in results {
            if track.isrc.as_ref() == Some(&isrc.to_string()) {
                return Some(track);
            }
        }

        None
    }

    async fn search_exact(&self, spotify_track: &SpotifyTrack) -> Option<TidalTrack> {
        let primary_artist = spotify_track.artists.first().map(|s| s.as_str()).unwrap_or("");
        let query = format!("{} {}", primary_artist, spotify_track.name);

        let results = self.search_tracks(&query).await.ok()?;

        for track in results {
            if is_exact_match(spotify_track, &track) {
                return Some(track);
            }
        }

        None
    }

    async fn search_fuzzy(&self, spotify_track: &SpotifyTrack) -> Option<TidalTrack> {
        let primary_artist = spotify_track.artists.first().map(|s| s.as_str()).unwrap_or("");
        let query = format!("{} {}", primary_artist, spotify_track.name);

        let results = self.search_tracks(&query).await.ok()?;

        let mut best_match: Option<TidalTrack> = None;
        let mut best_score: f64 = 0.0;

        for track in results.into_iter().take(10) {
            let score = calculate_similarity(spotify_track, &track);
            if score > best_score && is_fuzzy_match(score) {
                best_score = score;
                best_match = Some(track);
            }
        }

        if best_match.is_some() {
            debug!(
                "Fuzzy match found for {} with score {:.2}",
                spotify_track.name, best_score
            );
        }

        best_match
    }

    async fn search_tracks(&self, query: &str) -> Result<Vec<TidalTrack>> {
        let url = format!("{}/searchresults/{}/relationships/tracks", TIDAL_API_BASE, urlencoding::encode(query));

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&self.access_token)
            .query(&[("countryCode", "US"), ("limit", "20")])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            warn!("Tidal search failed ({}): {}", status, error_text);
            return Ok(Vec::new());
        }

        let search_response: TidalSearchResponse = response.json().await.unwrap_or(TidalSearchResponse { tracks: None });

        Ok(search_response
            .tracks
            .unwrap_or_default()
            .into_iter()
            .map(|t| TidalTrack {
                id: t.id,
                name: t.title,
                artists: t.artists.into_iter().map(|a| a.name).collect(),
                album: t.album.map(|a| a.title).unwrap_or_default(),
                duration_secs: t.duration,
                isrc: t.isrc,
            })
            .collect())
    }

    pub async fn create_playlist(&self, name: &str, description: &str) -> Result<TidalPlaylist> {
        let url = format!("{}/users/{}/playlists", TIDAL_API_BASE, self.user_id);

        let request = CreatePlaylistRequest {
            name: name.to_string(),
            description: Some(description.to_string()),
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::TidalApi(format!(
                "Failed to create playlist: {}",
                error_text
            )));
        }

        let api_playlist: TidalApiPlaylist = response.json().await?;

        info!("Created Tidal playlist: {}", name);

        Ok(TidalPlaylist {
            id: api_playlist.uuid,
            name: api_playlist.name,
            description: api_playlist.description.unwrap_or_default(),
            tracks: Vec::new(),
        })
    }

    pub async fn add_tracks_to_playlist(
        &self,
        playlist_id: &str,
        track_ids: &[u64],
    ) -> Result<bool> {
        if track_ids.is_empty() {
            return Ok(true);
        }

        let url = format!(
            "{}/playlists/{}/relationships/tracks",
            TIDAL_API_BASE, playlist_id
        );

        let track_data: Vec<_> = track_ids
            .iter()
            .map(|id| serde_json::json!({"id": id.to_string(), "type": "tracks"}))
            .collect();

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({"data": track_data}))
            .send()
            .await?;

        if response.status().is_success() {
            info!("Added {} tracks to playlist", track_ids.len());
            Ok(true)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            warn!("Failed to add tracks to playlist: {}", error_text);
            Ok(false)
        }
    }
}
