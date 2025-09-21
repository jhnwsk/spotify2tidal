import os
from typing import Optional
from dotenv import load_dotenv

class Config:
    def __init__(self):
        load_dotenv()

        self.spotify_client_id = os.getenv('SPOTIFY_CLIENT_ID')
        self.spotify_client_secret = os.getenv('SPOTIFY_CLIENT_SECRET')
        self.spotify_redirect_uri = os.getenv('SPOTIFY_REDIRECT_URI', 'http://127.0.0.1:8080/callback')

        # Tidal now uses OAuth - no credentials needed

    def validate_spotify_config(self) -> bool:
        return bool(self.spotify_client_id and self.spotify_client_secret)

    def validate_tidal_config(self) -> bool:
        # Tidal uses OAuth now - always valid
        return True

    def get_missing_config(self) -> list[str]:
        missing = []

        if not self.spotify_client_id:
            missing.append('SPOTIFY_CLIENT_ID')
        if not self.spotify_client_secret:
            missing.append('SPOTIFY_CLIENT_SECRET')
        # Tidal uses OAuth - no config needed

        return missing