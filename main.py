#!/usr/bin/env python3

import click
import sys
from pathlib import Path
from colorama import init, Fore, Style

from src.config import Config
from src.migrator import PlaylistMigrator
from src.logger import setup_logger

init()

@click.group()
@click.option('--verbose', '-v', is_flag=True, help='Enable verbose logging')
@click.pass_context
def cli(ctx, verbose):
    ctx.ensure_object(dict)
    ctx.obj['verbose'] = verbose

    if verbose:
        import logging
        logging.getLogger().setLevel(logging.DEBUG)

@cli.command()
@click.option('--dry-run', is_flag=True, help='Preview migration without creating playlists')
@click.pass_context
def migrate_all(ctx, dry_run):
    """Migrate all your Spotify playlists to Tidal."""

    print(f"{Fore.CYAN}🎵 Tidal Playlist Migrator{Style.RESET_ALL}")
    print("=" * 50)

    if dry_run:
        print(f"{Fore.YELLOW}⚠️  DRY RUN MODE - No playlists will be created{Style.RESET_ALL}")

    try:
        config = Config()
        missing_config = config.get_missing_config()

        if missing_config:
            print(f"{Fore.RED}❌ Missing configuration:{Style.RESET_ALL}")
            for item in missing_config:
                print(f"   - {item}")
            print(f"\n{Fore.YELLOW}Please copy .env.example to .env and fill in your credentials.{Style.RESET_ALL}")
            sys.exit(1)

        migrator = PlaylistMigrator(config)
        results = migrator.migrate_all_playlists(dry_run=dry_run)

        if not dry_run:
            print(f"\n{Fore.GREEN}✅ Migration completed!{Style.RESET_ALL}")
        else:
            print(f"\n{Fore.YELLOW}📋 Dry run completed - no changes made{Style.RESET_ALL}")

    except Exception as e:
        print(f"{Fore.RED}❌ Migration failed: {e}{Style.RESET_ALL}")
        if ctx.obj['verbose']:
            import traceback
            traceback.print_exc()
        sys.exit(1)

@cli.command()
@click.argument('playlist_names', nargs=-1, required=True)
@click.option('--dry-run', is_flag=True, help='Preview migration without creating playlists')
@click.pass_context
def migrate(ctx, playlist_names, dry_run):
    """Migrate specific playlists to Tidal."""

    print(f"{Fore.CYAN}🎵 Tidal Playlist Migrator{Style.RESET_ALL}")
    print("=" * 50)

    if dry_run:
        print(f"{Fore.YELLOW}⚠️  DRY RUN MODE - No playlists will be created{Style.RESET_ALL}")

    print(f"Target playlists: {', '.join(playlist_names)}")

    try:
        config = Config()
        missing_config = config.get_missing_config()

        if missing_config:
            print(f"{Fore.RED}❌ Missing configuration:{Style.RESET_ALL}")
            for item in missing_config:
                print(f"   - {item}")
            print(f"\n{Fore.YELLOW}Please copy .env.example to .env and fill in your credentials.{Style.RESET_ALL}")
            sys.exit(1)

        migrator = PlaylistMigrator(config)
        results = migrator.migrate_specific_playlists(list(playlist_names), dry_run=dry_run)

        if not dry_run:
            print(f"\n{Fore.GREEN}✅ Migration completed!{Style.RESET_ALL}")
        else:
            print(f"\n{Fore.YELLOW}📋 Dry run completed - no changes made{Style.RESET_ALL}")

    except Exception as e:
        print(f"{Fore.RED}❌ Migration failed: {e}{Style.RESET_ALL}")
        if ctx.obj['verbose']:
            import traceback
            traceback.print_exc()
        sys.exit(1)

@cli.command()
@click.pass_context
def list_playlists(ctx):
    """List all your Spotify playlists."""

    print(f"{Fore.CYAN}📋 Your Spotify Playlists{Style.RESET_ALL}")
    print("=" * 50)

    try:
        config = Config()

        if not config.validate_spotify_config():
            print(f"{Fore.RED}❌ Missing Spotify configuration{Style.RESET_ALL}")
            missing = [item for item in config.get_missing_config() if 'SPOTIFY' in item]
            for item in missing:
                print(f"   - {item}")
            sys.exit(1)

        from src.spotify_client import SpotifyClient
        spotify_client = SpotifyClient(config)
        playlists = spotify_client.get_user_playlists()

        if not playlists:
            print(f"{Fore.YELLOW}No playlists found{Style.RESET_ALL}")
            return

        for i, playlist in enumerate(playlists, 1):
            print(f"{i:2d}. {Fore.GREEN}{playlist.name}{Style.RESET_ALL} ({playlist.total_tracks} tracks)")
            if playlist.description:
                print(f"     {Fore.CYAN}{playlist.description}{Style.RESET_ALL}")

        print(f"\n{Fore.CYAN}Total: {len(playlists)} playlists{Style.RESET_ALL}")

    except Exception as e:
        print(f"{Fore.RED}❌ Failed to list playlists: {e}{Style.RESET_ALL}")
        if ctx.obj['verbose']:
            import traceback
            traceback.print_exc()
        sys.exit(1)

@cli.command()
@click.pass_context
def setup(ctx):
    """Setup guide for first-time users."""

    print(f"{Fore.CYAN}🔧 Tidal Migrator Setup Guide{Style.RESET_ALL}")
    print("=" * 50)

    print(f"\n{Fore.YELLOW}1. Spotify API Setup{Style.RESET_ALL}")
    print("   • Go to https://developer.spotify.com/dashboard/")
    print("   • Create a new app")
    print("   • Copy your Client ID and Client Secret")
    print("   • Add 'http://127.0.0.1:8080/callback' as a redirect URI")

    print(f"\n{Fore.YELLOW}2. Configuration{Style.RESET_ALL}")
    print("   • Copy .env.example to .env")
    print("   • Fill in your Spotify credentials")
    print("   • Fill in your Tidal username and password")

    print(f"\n{Fore.YELLOW}3. Installation{Style.RESET_ALL}")
    print("   • pip install -r requirements.txt")

    print(f"\n{Fore.YELLOW}4. Usage{Style.RESET_ALL}")
    print("   • python main.py list-playlists  (to see your playlists)")
    print("   • python main.py migrate-all --dry-run  (to test migration)")
    print("   • python main.py migrate-all  (to perform migration)")

    print(f"\n{Fore.GREEN}Ready to start migrating! 🎵{Style.RESET_ALL}")

if __name__ == '__main__':
    cli()