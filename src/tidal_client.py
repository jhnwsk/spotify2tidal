from typing import List, Dict, Optional, Any
import tidalapi
from dataclasses import dataclass
from fuzzywuzzy import fuzz
from colorama import Fore, Style
from .config import Config
from .logger import setup_logger
from .spotify_client import SpotifyTrack

@dataclass
class TidalTrack:
    id: int
    name: str
    artists: List[str]
    album: str
    duration: int
    isrc: Optional[str] = None

@dataclass
class TidalPlaylist:
    id: str
    name: str
    description: str
    tracks: List[TidalTrack]

@dataclass
class MatchResult:
    spotify_track: SpotifyTrack
    tidal_track: Optional[TidalTrack]
    match_score: float
    match_method: str
    success: bool

class TidalClient:
    def __init__(self, config: Config):
        self.config = config
        self.logger = setup_logger('tidal_client')
        self.session = None
        self._authenticate()

    def _authenticate(self):
        try:
            self.session = tidalapi.Session()

            # Check if already logged in
            if self.session.check_login():
                user = self.session.user
                if user:
                    # Try different attributes for user identification
                    user_id = getattr(user, 'id', getattr(user, 'username', 'Unknown User'))
                    self.logger.info(f"Using existing Tidal session for user: {user_id}")
                    return

            # If no existing session, start OAuth flow
            self.logger.info("Starting Tidal OAuth authentication...")
            print(f"\n{Fore.CYAN}🔐 Tidal Authentication Required{Style.RESET_ALL}")
            print("You need to authenticate with Tidal using OAuth.")
            print("This is a one-time setup - your session will be saved for future use.")
            print("\nStarting authentication process...")

            login, future = self.session.login_oauth()

            print(f"\n{Fore.YELLOW}📱 Please open this URL in your browser:{Style.RESET_ALL}")
            print(f"{Fore.BLUE}{login.verification_uri_complete}{Style.RESET_ALL}")
            print(f"\n{Fore.CYAN}Or visit: {login.verification_uri}{Style.RESET_ALL}")
            print(f"{Fore.CYAN}And enter code: {login.user_code}{Style.RESET_ALL}")
            print(f"\n{Fore.GREEN}Waiting for authentication...{Style.RESET_ALL}")

            # Wait for authentication to complete
            future.result()

            if self.session.check_login():
                user = self.session.user
                # Try different attributes for user identification
                user_id = getattr(user, 'id', getattr(user, 'username', 'Unknown User'))
                self.logger.info(f"Successfully authenticated as Tidal user: {user_id}")
                print(f"{Fore.GREEN}✅ Authentication successful!{Style.RESET_ALL}\n")
            else:
                raise Exception("OAuth authentication failed")

        except Exception as e:
            self.logger.error(f"Failed to authenticate with Tidal: {e}")
            raise

    def search_track(self, track: SpotifyTrack) -> Optional[TidalTrack]:
        if not self.session:
            return None

        try:
            if track.isrc:
                result = self._search_by_isrc(track.isrc)
                if result:
                    self.logger.debug(f"Found track by ISRC: {track.name}")
                    return result

            result = self._search_by_artist_and_title(track)
            if result:
                return result

            result = self._fuzzy_search(track)
            return result

        except Exception as e:
            self.logger.error(f"Error searching for track {track.name}: {e}")
            return None

    def _search_by_isrc(self, isrc: str) -> Optional[TidalTrack]:
        try:
            search_results = self.session.search(query=isrc, models=[tidalapi.Track])
            if search_results and 'tracks' in search_results:
                for track in search_results['tracks']:
                    if hasattr(track, 'isrc') and track.isrc == isrc:
                        return self._convert_tidal_track(track)
        except Exception as e:
            self.logger.debug(f"ISRC search failed for {isrc}: {e}")

        return None

    def _search_by_artist_and_title(self, spotify_track: SpotifyTrack) -> Optional[TidalTrack]:
        try:
            primary_artist = spotify_track.artists[0] if spotify_track.artists else ""
            query = f"{primary_artist} {spotify_track.name}"

            search_results = self.session.search(query=query, models=[tidalapi.Track])

            if search_results and 'tracks' in search_results:
                for track in search_results['tracks']:
                    if self._is_exact_match(spotify_track, track):
                        return self._convert_tidal_track(track)

        except Exception as e:
            self.logger.debug(f"Artist/title search failed for {spotify_track.name}: {e}")

        return None

    def _fuzzy_search(self, spotify_track: SpotifyTrack) -> Optional[TidalTrack]:
        try:
            primary_artist = spotify_track.artists[0] if spotify_track.artists else ""
            query = f"{primary_artist} {spotify_track.name}"

            search_results = self.session.search(query=query, models=[tidalapi.Track])

            if search_results and 'tracks' in search_results:
                best_match = None
                best_score = 0

                for track in search_results['tracks'][:10]:
                    score = self._calculate_similarity_score(spotify_track, track)
                    if score > best_score and score > 85:
                        best_score = score
                        best_match = track

                if best_match:
                    self.logger.debug(f"Fuzzy match found for {spotify_track.name} with score {best_score}")
                    return self._convert_tidal_track(best_match)

        except Exception as e:
            self.logger.debug(f"Fuzzy search failed for {spotify_track.name}: {e}")

        return None

    def _is_exact_match(self, spotify_track: SpotifyTrack, tidal_track) -> bool:
        spotify_name = spotify_track.name.lower().strip()
        tidal_name = tidal_track.name.lower().strip()

        spotify_artist = spotify_track.artists[0].lower().strip() if spotify_track.artists else ""
        tidal_artist = tidal_track.artist.name.lower().strip() if tidal_track.artist else ""

        name_match = spotify_name == tidal_name
        artist_match = spotify_artist == tidal_artist

        duration_diff = abs(spotify_track.duration_ms / 1000 - tidal_track.duration) if tidal_track.duration else float('inf')
        duration_match = duration_diff < 5

        return name_match and artist_match and duration_match

    def _calculate_similarity_score(self, spotify_track: SpotifyTrack, tidal_track) -> float:
        name_score = fuzz.ratio(spotify_track.name.lower(), tidal_track.name.lower())

        spotify_artist = spotify_track.artists[0].lower() if spotify_track.artists else ""
        tidal_artist = tidal_track.artist.name.lower() if tidal_track.artist else ""
        artist_score = fuzz.ratio(spotify_artist, tidal_artist)

        album_score = fuzz.ratio(spotify_track.album.lower(), tidal_track.album.name.lower() if tidal_track.album else "")

        duration_score = 100
        if tidal_track.duration:
            duration_diff = abs(spotify_track.duration_ms / 1000 - tidal_track.duration)
            if duration_diff < 5:
                duration_score = 100
            elif duration_diff < 15:
                duration_score = 80
            else:
                duration_score = 50

        total_score = (name_score * 0.4 + artist_score * 0.4 + album_score * 0.1 + duration_score * 0.1)
        return total_score

    def _convert_tidal_track(self, tidal_track) -> TidalTrack:
        return TidalTrack(
            id=tidal_track.id,
            name=tidal_track.name,
            artists=[tidal_track.artist.name] if tidal_track.artist else [],
            album=tidal_track.album.name if tidal_track.album else "",
            duration=tidal_track.duration or 0,
            isrc=getattr(tidal_track, 'isrc', None)
        )

    def create_playlist(self, name: str, description: str = "") -> Optional[TidalPlaylist]:
        try:
            playlist = self.session.user.create_playlist(name, description)
            if playlist:
                self.logger.info(f"Created Tidal playlist: {name}")
                return TidalPlaylist(
                    id=playlist.id,
                    name=playlist.name,
                    description=playlist.description or "",
                    tracks=[]
                )
        except Exception as e:
            self.logger.error(f"Failed to create playlist {name}: {e}")

        return None

    def add_tracks_to_playlist(self, playlist_id: str, track_ids: List[int]) -> bool:
        try:
            playlist = self.session.playlist(playlist_id)
            if playlist and track_ids:
                playlist.add(track_ids)
                self.logger.info(f"Added {len(track_ids)} tracks to playlist")
                return True
        except Exception as e:
            self.logger.error(f"Failed to add tracks to playlist {playlist_id}: {e}")

        return False