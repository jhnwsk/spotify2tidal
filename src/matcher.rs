use strsim::jaro_winkler;

use crate::spotify::SpotifyTrack;
use crate::tidal::{MatchMethod, TidalTrack};

const FUZZY_THRESHOLD: f64 = 0.85;

/// Calculate similarity score between a Spotify track and a Tidal track.
/// Uses weighted scoring: 40% name + 40% artist + 10% album + 10% duration
pub fn calculate_similarity(spotify: &SpotifyTrack, tidal: &TidalTrack) -> f64 {
    let name_score = jaro_winkler(&spotify.name.to_lowercase(), &tidal.name.to_lowercase());

    let artist_score = match (spotify.artists.first(), tidal.artists.first()) {
        (Some(s_artist), Some(t_artist)) => {
            jaro_winkler(&s_artist.to_lowercase(), &t_artist.to_lowercase())
        }
        _ => 0.0,
    };

    let album_score = jaro_winkler(&spotify.album.to_lowercase(), &tidal.album.to_lowercase());

    // Duration score: within 5s = 100%, within 15s = 80%, else 50%
    let spotify_duration_secs = spotify.duration_ms / 1000;
    let duration_diff = (spotify_duration_secs as i64 - tidal.duration_secs as i64).abs();
    let duration_score = match duration_diff {
        0..=5 => 1.0,
        6..=15 => 0.8,
        _ => 0.5,
    };

    // Weighted average (same as Python: 40% name + 40% artist + 10% album + 10% duration)
    name_score * 0.4 + artist_score * 0.4 + album_score * 0.1 + duration_score * 0.1
}

/// Check if a similarity score meets the fuzzy match threshold (85%)
pub fn is_fuzzy_match(score: f64) -> bool {
    score >= FUZZY_THRESHOLD
}

/// Check if two tracks are an exact match (name, artist, and duration within 5 seconds)
pub fn is_exact_match(spotify: &SpotifyTrack, tidal: &TidalTrack) -> bool {
    let spotify_name = spotify.name.to_lowercase();
    let tidal_name = tidal.name.to_lowercase();

    let spotify_artist = spotify
        .artists
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();
    let tidal_artist = tidal
        .artists
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    let name_match = spotify_name.trim() == tidal_name.trim();
    let artist_match = spotify_artist.trim() == tidal_artist.trim();

    let spotify_duration_secs = spotify.duration_ms / 1000;
    let duration_diff = (spotify_duration_secs as i64 - tidal.duration_secs as i64).abs();
    let duration_match = duration_diff < 5;

    name_match && artist_match && duration_match
}

/// Check if two tracks have matching ISRCs
pub fn is_isrc_match(spotify: &SpotifyTrack, tidal: &TidalTrack) -> bool {
    match (&spotify.isrc, &tidal.isrc) {
        (Some(s_isrc), Some(t_isrc)) => s_isrc == t_isrc,
        _ => false,
    }
}

/// Determine the best match method for a track pair
pub fn determine_match_method(spotify: &SpotifyTrack, tidal: &TidalTrack) -> MatchMethod {
    if is_isrc_match(spotify, tidal) {
        MatchMethod::Isrc
    } else if is_exact_match(spotify, tidal) {
        MatchMethod::Exact
    } else if is_fuzzy_match(calculate_similarity(spotify, tidal)) {
        MatchMethod::Fuzzy
    } else {
        MatchMethod::NoMatch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isrc_match_exact() {
        let spotify = SpotifyTrack {
            id: "1".to_string(),
            name: "Bohemian Rhapsody".to_string(),
            artists: vec!["Queen".to_string()],
            album: "A Night at the Opera".to_string(),
            duration_ms: 354000,
            isrc: Some("GBUM71029604".to_string()),
            popularity: 85,
        };

        let tidal = TidalTrack {
            id: 1000,
            name: "Bohemian Rhapsody".to_string(),
            artists: vec!["Queen".to_string()],
            album: "A Night at the Opera".to_string(),
            duration_secs: 354,
            isrc: Some("GBUM71029604".to_string()),
        };

        assert!(is_isrc_match(&spotify, &tidal));
        assert_eq!(determine_match_method(&spotify, &tidal), MatchMethod::Isrc);
    }

    #[test]
    fn test_exact_match_case_insensitive() {
        let spotify = SpotifyTrack {
            id: "1".to_string(),
            name: "Don't Stop Me Now".to_string(),
            artists: vec!["Queen".to_string()],
            album: "Jazz".to_string(),
            duration_ms: 209000,
            isrc: None,
            popularity: 80,
        };

        let tidal = TidalTrack {
            id: 1000,
            name: "don't stop me now".to_string(),
            artists: vec!["queen".to_string()],
            album: "Jazz".to_string(),
            duration_secs: 209,
            isrc: None,
        };

        assert!(is_exact_match(&spotify, &tidal));
    }

    #[test]
    fn test_fuzzy_match_similar_names() {
        let spotify = SpotifyTrack {
            id: "1".to_string(),
            name: "Don't Stop Me Now".to_string(),
            artists: vec!["Queen".to_string()],
            album: "Jazz".to_string(),
            duration_ms: 209000,
            isrc: None,
            popularity: 80,
        };

        let tidal = TidalTrack {
            id: 1000,
            name: "Dont Stop Me Now".to_string(), // Missing apostrophe
            artists: vec!["Queen".to_string()],
            album: "Jazz".to_string(),
            duration_secs: 209,
            isrc: None,
        };

        let score = calculate_similarity(&spotify, &tidal);
        assert!(score > 0.85, "Score {} should be > 0.85", score);
    }

    #[test]
    fn test_no_match_different_songs() {
        let spotify = SpotifyTrack {
            id: "1".to_string(),
            name: "Bohemian Rhapsody".to_string(),
            artists: vec!["Queen".to_string()],
            album: "A Night at the Opera".to_string(),
            duration_ms: 354000,
            isrc: None,
            popularity: 85,
        };

        let tidal = TidalTrack {
            id: 1000,
            name: "Stairway to Heaven".to_string(),
            artists: vec!["Led Zeppelin".to_string()],
            album: "Led Zeppelin IV".to_string(),
            duration_secs: 482,
            isrc: None,
        };

        let score = calculate_similarity(&spotify, &tidal);
        assert!(score < 0.85, "Score {} should be < 0.85", score);
        assert_eq!(
            determine_match_method(&spotify, &tidal),
            MatchMethod::NoMatch
        );
    }

    #[test]
    fn test_duration_tolerance() {
        let spotify = SpotifyTrack {
            id: "1".to_string(),
            name: "Test Song".to_string(),
            artists: vec!["Test Artist".to_string()],
            album: "Test Album".to_string(),
            duration_ms: 180000, // 180 seconds
            isrc: None,
            popularity: 50,
        };

        // Within 5 seconds - should match
        let tidal_close = TidalTrack {
            id: 1000,
            name: "Test Song".to_string(),
            artists: vec!["Test Artist".to_string()],
            album: "Test Album".to_string(),
            duration_secs: 183, // 3 seconds difference
            isrc: None,
        };
        assert!(is_exact_match(&spotify, &tidal_close));

        // Beyond 5 seconds - should not exact match
        let tidal_far = TidalTrack {
            id: 1001,
            name: "Test Song".to_string(),
            artists: vec!["Test Artist".to_string()],
            album: "Test Album".to_string(),
            duration_secs: 190, // 10 seconds difference
            isrc: None,
        };
        assert!(!is_exact_match(&spotify, &tidal_far));
    }
}
