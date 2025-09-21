from typing import List, Dict, Optional, Tuple
import json
from pathlib import Path
from datetime import datetime
from tqdm import tqdm
from dataclasses import dataclass, asdict

from .spotify_client import SpotifyClient, SpotifyPlaylist, SpotifyTrack
from .tidal_client import TidalClient, TidalTrack, TidalPlaylist
from .config import Config
from .logger import setup_logger

@dataclass
class MigrationResult:
    playlist_name: str
    total_tracks: int
    successful_matches: int
    failed_matches: int
    success_rate: float
    failed_tracks: List[Dict[str, str]]
    tidal_playlist_id: Optional[str] = None

@dataclass
class TrackMigrationResult:
    spotify_track: SpotifyTrack
    tidal_track: Optional[TidalTrack]
    success: bool
    match_method: str
    match_score: float

class PlaylistMigrator:
    def __init__(self, config: Config):
        self.config = config
        self.logger = setup_logger('migrator')
        self.spotify_client = SpotifyClient(config)
        self.tidal_client = TidalClient(config)

    def migrate_all_playlists(self, dry_run: bool = False) -> List[MigrationResult]:
        playlists = self.spotify_client.get_user_playlists()
        results = []

        self.logger.info(f"Starting migration of {len(playlists)} playlists (dry_run={dry_run})")

        for playlist in tqdm(playlists, desc="Migrating playlists"):
            result = self.migrate_playlist(playlist, dry_run)
            results.append(result)

        self._save_migration_results(results)
        self._print_summary(results)

        return results

    def migrate_playlist(self, spotify_playlist: SpotifyPlaylist, dry_run: bool = False) -> MigrationResult:
        self.logger.info(f"Migrating playlist: {spotify_playlist.name}")

        track_results = []
        successful_matches = 0
        failed_matches = 0
        failed_tracks = []

        for track in tqdm(spotify_playlist.tracks, desc=f"Processing {spotify_playlist.name}", leave=False):
            tidal_track = self.tidal_client.search_track(track)

            if tidal_track:
                successful_matches += 1
                match_method = self._determine_match_method(track, tidal_track)
                match_score = self._calculate_match_score(track, tidal_track)

                track_result = TrackMigrationResult(
                    spotify_track=track,
                    tidal_track=tidal_track,
                    success=True,
                    match_method=match_method,
                    match_score=match_score
                )
            else:
                failed_matches += 1
                failed_tracks.append({
                    'name': track.name,
                    'artist': ', '.join(track.artists),
                    'album': track.album
                })

                track_result = TrackMigrationResult(
                    spotify_track=track,
                    tidal_track=None,
                    success=False,
                    match_method="no_match",
                    match_score=0.0
                )

            track_results.append(track_result)

        success_rate = (successful_matches / len(spotify_playlist.tracks)) * 100 if spotify_playlist.tracks else 0

        tidal_playlist_id = None
        if not dry_run and successful_matches > 0:
            tidal_playlist_id = self._create_tidal_playlist(spotify_playlist, track_results)

        result = MigrationResult(
            playlist_name=spotify_playlist.name,
            total_tracks=len(spotify_playlist.tracks),
            successful_matches=successful_matches,
            failed_matches=failed_matches,
            success_rate=success_rate,
            failed_tracks=failed_tracks,
            tidal_playlist_id=tidal_playlist_id
        )

        self.logger.info(
            f"Playlist migration completed: {spotify_playlist.name} - "
            f"{successful_matches}/{len(spotify_playlist.tracks)} tracks matched "
            f"({success_rate:.1f}% success rate)"
        )

        return result

    def _determine_match_method(self, spotify_track: SpotifyTrack, tidal_track: TidalTrack) -> str:
        if spotify_track.isrc and tidal_track.isrc and spotify_track.isrc == tidal_track.isrc:
            return "isrc"

        spotify_name = spotify_track.name.lower().strip()
        tidal_name = tidal_track.name.lower().strip()
        spotify_artist = spotify_track.artists[0].lower().strip() if spotify_track.artists else ""
        tidal_artist = tidal_track.artists[0].lower().strip() if tidal_track.artists else ""

        if spotify_name == tidal_name and spotify_artist == tidal_artist:
            return "exact"

        return "fuzzy"

    def _calculate_match_score(self, spotify_track: SpotifyTrack, tidal_track: TidalTrack) -> float:
        from fuzzywuzzy import fuzz

        name_score = fuzz.ratio(spotify_track.name.lower(), tidal_track.name.lower())
        artist_score = fuzz.ratio(
            spotify_track.artists[0].lower() if spotify_track.artists else "",
            tidal_track.artists[0].lower() if tidal_track.artists else ""
        )

        return (name_score + artist_score) / 2

    def _create_tidal_playlist(self, spotify_playlist: SpotifyPlaylist, track_results: List[TrackMigrationResult]) -> Optional[str]:
        successful_tracks = [tr for tr in track_results if tr.success and tr.tidal_track]

        if not successful_tracks:
            return None

        try:
            tidal_playlist = self.tidal_client.create_playlist(
                name=spotify_playlist.name,
                description=f"Migrated from Spotify. {spotify_playlist.description}"
            )

            if not tidal_playlist:
                return None

            track_ids = [tr.tidal_track.id for tr in successful_tracks]

            batch_size = 100
            for i in range(0, len(track_ids), batch_size):
                batch = track_ids[i:i + batch_size]
                success = self.tidal_client.add_tracks_to_playlist(tidal_playlist.id, batch)

                if not success:
                    self.logger.warning(f"Failed to add batch {i//batch_size + 1} to playlist {spotify_playlist.name}")

            return tidal_playlist.id

        except Exception as e:
            self.logger.error(f"Failed to create Tidal playlist for {spotify_playlist.name}: {e}")
            return None

    def _save_migration_results(self, results: List[MigrationResult]):
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        results_dir = Path('migration_results')
        results_dir.mkdir(exist_ok=True)

        filename = results_dir / f'migration_results_{timestamp}.json'

        results_dict = [asdict(result) for result in results]

        with open(filename, 'w', encoding='utf-8') as f:
            json.dump(results_dict, f, indent=2, ensure_ascii=False)

        self.logger.info(f"Migration results saved to: {filename}")

    def _print_summary(self, results: List[MigrationResult]):
        total_playlists = len(results)
        total_tracks = sum(r.total_tracks for r in results)
        total_successful = sum(r.successful_matches for r in results)
        total_failed = sum(r.failed_matches for r in results)

        overall_success_rate = (total_successful / total_tracks) * 100 if total_tracks > 0 else 0

        print("\n" + "="*60)
        print("MIGRATION SUMMARY")
        print("="*60)
        print(f"Total playlists processed: {total_playlists}")
        print(f"Total tracks processed: {total_tracks}")
        print(f"Successfully matched: {total_successful}")
        print(f"Failed to match: {total_failed}")
        print(f"Overall success rate: {overall_success_rate:.1f}%")
        print("="*60)

        print("\nPlaylist breakdown:")
        for result in results:
            print(f"  {result.playlist_name}: {result.successful_matches}/{result.total_tracks} "
                  f"({result.success_rate:.1f}%)")

        if any(r.failed_tracks for r in results):
            print(f"\nFailed tracks have been logged and saved to migration_results/")

    def migrate_specific_playlists(self, playlist_names: List[str], dry_run: bool = False) -> List[MigrationResult]:
        all_playlists = self.spotify_client.get_user_playlists()

        target_playlists = []
        for name in playlist_names:
            playlist = next((p for p in all_playlists if p.name.lower() == name.lower()), None)
            if playlist:
                target_playlists.append(playlist)
            else:
                self.logger.warning(f"Playlist not found: {name}")

        if not target_playlists:
            self.logger.error("No valid playlists found to migrate")
            return []

        results = []
        for playlist in target_playlists:
            result = self.migrate_playlist(playlist, dry_run)
            results.append(result)

        self._save_migration_results(results)
        self._print_summary(results)

        return results