from typing import List, Dict, Optional, Any
import spotipy
from spotipy.oauth2 import SpotifyOAuth
from dataclasses import dataclass
from .config import Config
from .logger import setup_logger

@dataclass
class SpotifyTrack:
    id: str
    name: str
    artists: List[str]
    album: str
    duration_ms: int
    isrc: Optional[str] = None
    popularity: int = 0
    preview_url: Optional[str] = None

@dataclass
class SpotifyPlaylist:
    id: str
    name: str
    description: str
    tracks: List[SpotifyTrack]
    total_tracks: int
    public: bool
    owner: str

class SpotifyClient:
    def __init__(self, config: Config):
        self.config = config
        self.logger = setup_logger('spotify_client')
        self.sp = None
        self._authenticate()

    def _authenticate(self):
        try:
            auth_manager = SpotifyOAuth(
                client_id=self.config.spotify_client_id,
                client_secret=self.config.spotify_client_secret,
                redirect_uri=self.config.spotify_redirect_uri,
                scope="user-library-read playlist-read-private playlist-read-collaborative",
                cache_path=".spotify_cache"
            )

            self.sp = spotipy.Spotify(auth_manager=auth_manager)

            user = self.sp.current_user()
            self.logger.info(f"Successfully authenticated as Spotify user: {user['display_name']}")

        except Exception as e:
            self.logger.error(f"Failed to authenticate with Spotify: {e}")
            raise

    def get_user_playlists(self) -> List[SpotifyPlaylist]:
        playlists = []

        try:
            results = self.sp.current_user_playlists(limit=50)

            while results:
                for playlist_data in results['items']:
                    if playlist_data['owner']['id'] == self.sp.current_user()['id']:
                        playlist = self._get_playlist_details(playlist_data)
                        if playlist:
                            playlists.append(playlist)

                if results['next']:
                    results = self.sp.next(results)
                else:
                    break

        except Exception as e:
            self.logger.error(f"Failed to fetch user playlists: {e}")
            raise

        self.logger.info(f"Found {len(playlists)} user playlists")
        return playlists

    def _get_playlist_details(self, playlist_data: Dict[str, Any]) -> Optional[SpotifyPlaylist]:
        try:
            playlist_id = playlist_data['id']
            name = playlist_data['name']
            description = playlist_data.get('description', '')
            total_tracks = playlist_data['tracks']['total']
            public = playlist_data.get('public', False)
            owner = playlist_data['owner']['display_name']

            self.logger.info(f"Fetching tracks for playlist: {name} ({total_tracks} tracks)")

            tracks = self._get_playlist_tracks(playlist_id)

            return SpotifyPlaylist(
                id=playlist_id,
                name=name,
                description=description,
                tracks=tracks,
                total_tracks=total_tracks,
                public=public,
                owner=owner
            )

        except Exception as e:
            self.logger.error(f"Failed to get playlist details for {playlist_data.get('name', 'Unknown')}: {e}")
            return None

    def _get_playlist_tracks(self, playlist_id: str) -> List[SpotifyTrack]:
        tracks = []

        try:
            results = self.sp.playlist_tracks(playlist_id, limit=100)

            while results:
                for item in results['items']:
                    if item['track'] and item['track']['type'] == 'track':
                        track_data = item['track']

                        track = SpotifyTrack(
                            id=track_data['id'],
                            name=track_data['name'],
                            artists=[artist['name'] for artist in track_data['artists']],
                            album=track_data['album']['name'],
                            duration_ms=track_data['duration_ms'],
                            isrc=track_data.get('external_ids', {}).get('isrc'),
                            popularity=track_data.get('popularity', 0),
                            preview_url=track_data.get('preview_url')
                        )
                        tracks.append(track)

                if results['next']:
                    results = self.sp.next(results)
                else:
                    break

        except Exception as e:
            self.logger.error(f"Failed to fetch tracks for playlist {playlist_id}: {e}")

        return tracks

    def get_playlist_by_id(self, playlist_id: str) -> Optional[SpotifyPlaylist]:
        try:
            playlist_data = self.sp.playlist(playlist_id)
            return self._get_playlist_details(playlist_data)
        except Exception as e:
            self.logger.error(f"Failed to get playlist {playlist_id}: {e}")
            return None