# Spotify to Tidal Playlist Migrator (Rust)

A command-line tool to migrate your Spotify playlists to Tidal, written in Rust.

## Features

- Migrate all or specific playlists from Spotify to Tidal
- Three-tier track matching algorithm:
  1. **ISRC matching** - Exact identification using International Standard Recording Code
  2. **Exact matching** - Artist + track name + duration (within 5 seconds)
  3. **Fuzzy matching** - String similarity with 85% threshold
- Dry-run mode to preview migrations without creating playlists
- Progress bars and colored output
- JSON migration reports saved to `migration_results/`

## Installation

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Spotify Developer Account
- Tidal Developer Account

### Build

```bash
cargo build --release
```

The binary will be at `target/release/spotify2tidal`.

## Configuration

Create a `.env` file in the project root:

```env
SPOTIFY_CLIENT_ID=your_spotify_client_id
SPOTIFY_CLIENT_SECRET=your_spotify_client_secret
SPOTIFY_REDIRECT_URI=http://127.0.0.1:8080/callback

TIDAL_CLIENT_ID=your_tidal_client_id
TIDAL_CLIENT_SECRET=your_tidal_client_secret
```

### Getting API Credentials

**Spotify:**
1. Go to https://developer.spotify.com/dashboard/
2. Create a new app
3. Copy your Client ID and Client Secret
4. Add `http://127.0.0.1:8080/callback` as a redirect URI

**Tidal:**
1. Go to https://developer.tidal.com/
2. Create a new application
3. Copy your Client ID and Client Secret

## Usage

```bash
# Show setup guide
spotify2tidal setup

# List all your Spotify playlists
spotify2tidal list-playlists

# Migrate all playlists (dry run - no changes made)
spotify2tidal migrate-all --dry-run

# Migrate all playlists
spotify2tidal migrate-all

# Migrate specific playlists
spotify2tidal migrate "My Playlist" "Another Playlist"

# Enable verbose logging
spotify2tidal -v migrate-all
```

## Project Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library exports
├── config.rs            # Environment configuration
├── error.rs             # Error types
├── matcher.rs           # Track matching algorithms
├── spotify/
│   ├── mod.rs
│   ├── models.rs        # SpotifyTrack, SpotifyPlaylist
│   └── client.rs        # Spotify API client
├── tidal/
│   ├── mod.rs
│   ├── models.rs        # TidalTrack, TidalPlaylist, MatchResult
│   └── client.rs        # Tidal API client
└── migrator/
    ├── mod.rs
    ├── report.rs        # Migration result types
    └── orchestrator.rs  # Migration logic
```

## Running Tests

```bash
cargo test
```

## License

MIT
