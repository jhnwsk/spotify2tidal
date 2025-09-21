# 🎵 Tidal Playlist Migrator

A Python tool to migrate your playlists from Spotify to Tidal with intelligent track matching.

## ✨ Features

- **Smart Track Matching**: Uses ISRC codes, exact matching, and fuzzy matching to find tracks
- **Batch Migration**: Migrate all playlists or specific ones
- **Dry Run Mode**: Preview migrations without making changes
- **Progress Tracking**: Real-time progress bars and detailed logging
- **Match Analytics**: Detailed reports on success rates and failed matches
- **CLI Interface**: Easy-to-use command-line interface

## 🚀 Quick Start

### 1. Clone and Install

```bash
git clone <your-repo-url>
cd tidal-migrator
pip install -r requirements.txt
```

### 2. Setup Spotify API

1. Go to [Spotify Developer Dashboard](https://developer.spotify.com/dashboard/)
2. Create a new app
3. Copy your **Client ID** and **Client Secret**
4. Add `http://127.0.0.1:8080/callback` as a redirect URI in your app settings

### 3. Tidal Account Setup

1. Ensure you have an active [Tidal subscription](https://tidal.com/)
2. **Authentication**: The tool uses OAuth - you'll authenticate through your browser on first run
3. **Note**: This tool uses unofficial Tidal API access but authentication is secure via OAuth

### 4. Configure Credentials

```bash
cp .env.example .env
```

Edit `.env` with your credentials:

```env
# Spotify API Credentials
SPOTIFY_CLIENT_ID=your_spotify_client_id_here
SPOTIFY_CLIENT_SECRET=your_spotify_client_secret_here
SPOTIFY_REDIRECT_URI=http://127.0.0.1:8080/callback

# Tidal Authentication
# No credentials needed - OAuth will be used on first run
```

### 5. Run Migration

```bash
# List your Spotify playlists
python main.py list-playlists

# Test migration (dry run)
python main.py migrate-all --dry-run

# Migrate all playlists
python main.py migrate-all

# Migrate specific playlists
python main.py migrate "My Playlist" "Another Playlist"
```

## 📋 Commands

**Get Help:**
```bash
python main.py --help
python main.py COMMAND --help  # Help for specific commands
```

### List Playlists
```bash
python main.py list-playlists
```
Shows all your Spotify playlists with track counts.

### Migrate All Playlists
```bash
python main.py migrate-all [--dry-run]
```
Migrates all your owned Spotify playlists to Tidal.

### Migrate Specific Playlists
```bash
python main.py migrate "Playlist Name 1" "Playlist Name 2" [--dry-run]
```
Migrates only the specified playlists.

### Setup Guide
```bash
python main.py setup
```
Interactive setup guide for first-time users.

## 🔍 How Track Matching Works

The tool uses a three-tier matching strategy:

1. **ISRC Matching** (Most Accurate)
   - Uses International Standard Recording Codes when available
   - Guarantees exact track matches

2. **Exact Matching**
   - Matches artist name, track name, and duration
   - High confidence matches

3. **Fuzzy Matching**
   - Uses string similarity algorithms
   - Handles variations in spelling, punctuation, etc.
   - Only accepts matches above 85% similarity

## 📊 Migration Results

After migration, you'll get:

- **Console Summary**: Real-time progress and final statistics
- **Detailed Logs**: Saved to `logs/migration_TIMESTAMP.log`
- **JSON Report**: Saved to `migration_results/migration_results_TIMESTAMP.json`

Example output:
```
=============================================================
MIGRATION SUMMARY
=============================================================
Total playlists processed: 5
Total tracks processed: 847
Successfully matched: 789
Failed to match: 58
Overall success rate: 93.2%
=============================================================

Playlist breakdown:
  Rock Classics: 45/47 (95.7%)
  Chill Vibes: 132/134 (98.5%)
  Workout Mix: 67/72 (93.1%)
```

## ⚠️ Important Notes

### Tidal API Usage
This tool uses the unofficial `tidalapi` Python library. While it works well, it's not officially supported by Tidal. Use at your own discretion.

### Rate Limiting
The tool includes built-in delays to avoid hitting API rate limits, but large libraries may take time to migrate.

### Track Availability
Not all Spotify tracks are available on Tidal. The tool will report unmatched tracks for manual review.

## 🔧 Troubleshooting

### Authentication Issues

**Spotify Authentication Failed**
- Verify your Client ID and Client Secret
- Ensure redirect URI is exactly `http://127.0.0.1:8080/callback`
- Check that your app has the correct permissions

**Tidal Authentication Failed**
- Verify your username and password
- Ensure you have an active Tidal subscription
- Try logging in through the Tidal website first

### Migration Issues

**Low Match Rates**
- Some genres/regions may have lower availability on Tidal
- Check the failed tracks list in the migration results
- Consider manually adding missing tracks

**Slow Performance**
- Large playlists take time due to API rate limiting
- Use specific playlist migration for faster results
- Run during off-peak hours for better API performance

## 📁 Project Structure

```
tidal-migrator/
├── src/
│   ├── config.py          # Configuration management
│   ├── spotify_client.py  # Spotify API wrapper
│   ├── tidal_client.py    # Tidal API wrapper
│   ├── migrator.py        # Core migration logic
│   └── logger.py          # Logging setup
├── main.py                # CLI interface
├── requirements.txt       # Dependencies
├── .env.example          # Configuration template
└── README.md             # This file
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 📄 License

This project is for educational and personal use. Respect the terms of service of both Spotify and Tidal.

## 🙏 Acknowledgments

- [Spotipy](https://github.com/plamere/spotipy) - Spotify Web API wrapper
- [tidalapi](https://github.com/tamland/python-tidal) - Unofficial Tidal API wrapper
- [Click](https://click.palletsprojects.com/) - Command line interface framework
