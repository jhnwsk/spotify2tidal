use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use spotify2tidal::{Config, PlaylistMigrator, SpotifyClient};

#[derive(Parser)]
#[command(name = "spotify2tidal")]
#[command(about = "Migrate Spotify playlists to Tidal")]
#[command(version)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Migrate all your Spotify playlists to Tidal
    MigrateAll {
        /// Preview migration without creating playlists
        #[arg(long)]
        dry_run: bool,

        /// Tidal client ID (or set TIDAL_CLIENT_ID env var)
        #[arg(long, env = "TIDAL_CLIENT_ID")]
        tidal_client_id: String,

        /// Tidal client secret (or set TIDAL_CLIENT_SECRET env var)
        #[arg(long, env = "TIDAL_CLIENT_SECRET")]
        tidal_client_secret: String,
    },

    /// Migrate specific playlists to Tidal
    Migrate {
        /// Names of playlists to migrate
        #[arg(required = true)]
        playlist_names: Vec<String>,

        /// Preview migration without creating playlists
        #[arg(long)]
        dry_run: bool,

        /// Tidal client ID (or set TIDAL_CLIENT_ID env var)
        #[arg(long, env = "TIDAL_CLIENT_ID")]
        tidal_client_id: String,

        /// Tidal client secret (or set TIDAL_CLIENT_SECRET env var)
        #[arg(long, env = "TIDAL_CLIENT_SECRET")]
        tidal_client_secret: String,
    },

    /// List all your Spotify playlists
    ListPlaylists,

    /// Show setup guide
    Setup,
}

fn setup_tracing(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_tracing(cli.verbose);

    match cli.command {
        Commands::MigrateAll {
            dry_run,
            tidal_client_id,
            tidal_client_secret,
        } => {
            migrate_all(dry_run, &tidal_client_id, &tidal_client_secret).await?;
        }
        Commands::Migrate {
            playlist_names,
            dry_run,
            tidal_client_id,
            tidal_client_secret,
        } => {
            migrate_specific(&playlist_names, dry_run, &tidal_client_id, &tidal_client_secret)
                .await?;
        }
        Commands::ListPlaylists => {
            list_playlists().await?;
        }
        Commands::Setup => {
            show_setup_guide();
        }
    }

    Ok(())
}

async fn migrate_all(dry_run: bool, tidal_client_id: &str, tidal_client_secret: &str) -> Result<()> {
    println!("{}", "Spotify to Tidal Playlist Migrator".cyan().bold());
    println!("{}", "=".repeat(50));

    if dry_run {
        println!(
            "{}",
            "DRY RUN MODE - No playlists will be created".yellow()
        );
    }

    let config = Config::from_env().context("Failed to load configuration")?;

    let missing = config.get_missing_config();
    if !missing.is_empty() {
        println!("{}", "Missing configuration:".red());
        for item in &missing {
            println!("   - {}", item);
        }
        println!(
            "\n{}",
            "Please copy .env.example to .env and fill in your credentials.".yellow()
        );
        std::process::exit(1);
    }

    let migrator = PlaylistMigrator::new(&config, tidal_client_id, tidal_client_secret)
        .await
        .context("Failed to initialize migrator")?;

    migrator.migrate_all_playlists(dry_run).await?;

    if !dry_run {
        println!("\n{}", "Migration completed!".green());
    } else {
        println!("\n{}", "Dry run completed - no changes made".yellow());
    }

    Ok(())
}

async fn migrate_specific(
    playlist_names: &[String],
    dry_run: bool,
    tidal_client_id: &str,
    tidal_client_secret: &str,
) -> Result<()> {
    println!("{}", "Spotify to Tidal Playlist Migrator".cyan().bold());
    println!("{}", "=".repeat(50));

    if dry_run {
        println!(
            "{}",
            "DRY RUN MODE - No playlists will be created".yellow()
        );
    }

    println!("Target playlists: {}", playlist_names.join(", "));

    let config = Config::from_env().context("Failed to load configuration")?;

    let missing = config.get_missing_config();
    if !missing.is_empty() {
        println!("{}", "Missing configuration:".red());
        for item in &missing {
            println!("   - {}", item);
        }
        std::process::exit(1);
    }

    let migrator = PlaylistMigrator::new(&config, tidal_client_id, tidal_client_secret)
        .await
        .context("Failed to initialize migrator")?;

    migrator
        .migrate_specific_playlists(playlist_names, dry_run)
        .await?;

    if !dry_run {
        println!("\n{}", "Migration completed!".green());
    } else {
        println!("\n{}", "Dry run completed - no changes made".yellow());
    }

    Ok(())
}

async fn list_playlists() -> Result<()> {
    println!("{}", "Your Spotify Playlists".cyan().bold());
    println!("{}", "=".repeat(50));

    let config = Config::from_env().context("Failed to load configuration")?;

    if !config.validate_spotify_config() {
        println!("{}", "Missing Spotify configuration".red());
        std::process::exit(1);
    }

    let spotify_client = SpotifyClient::new(&config)
        .await
        .context("Failed to connect to Spotify")?;

    let playlists = spotify_client
        .get_user_playlists()
        .await
        .context("Failed to fetch playlists")?;

    if playlists.is_empty() {
        println!("{}", "No playlists found".yellow());
        return Ok(());
    }

    for (i, playlist) in playlists.iter().enumerate() {
        println!(
            "{:2}. {} ({} tracks)",
            i + 1,
            playlist.name.green(),
            playlist.total_tracks
        );
        if !playlist.description.is_empty() {
            println!("     {}", playlist.description.cyan());
        }
    }

    println!("\n{}", format!("Total: {} playlists", playlists.len()).cyan());

    Ok(())
}

fn show_setup_guide() {
    println!("{}", "Spotify to Tidal Migrator Setup Guide".cyan().bold());
    println!("{}", "=".repeat(50));

    println!("\n{}", "1. Spotify API Setup".yellow());
    println!("   - Go to https://developer.spotify.com/dashboard/");
    println!("   - Create a new app");
    println!("   - Copy your Client ID and Client Secret");
    println!("   - Add 'http://127.0.0.1:8080/callback' as a redirect URI");

    println!("\n{}", "2. Tidal API Setup".yellow());
    println!("   - Go to https://developer.tidal.com/");
    println!("   - Create a new application");
    println!("   - Copy your Client ID and Client Secret");

    println!("\n{}", "3. Configuration".yellow());
    println!("   - Create a .env file with:");
    println!("     SPOTIFY_CLIENT_ID=your_spotify_client_id");
    println!("     SPOTIFY_CLIENT_SECRET=your_spotify_client_secret");
    println!("     SPOTIFY_REDIRECT_URI=http://127.0.0.1:8080/callback");
    println!("     TIDAL_CLIENT_ID=your_tidal_client_id");
    println!("     TIDAL_CLIENT_SECRET=your_tidal_client_secret");

    println!("\n{}", "4. Usage".yellow());
    println!("   - spotify2tidal list-playlists          (to see your playlists)");
    println!("   - spotify2tidal migrate-all --dry-run   (to test migration)");
    println!("   - spotify2tidal migrate-all             (to perform migration)");
    println!("   - spotify2tidal migrate \"Playlist Name\" (to migrate specific playlist)");

    println!("\n{}", "Ready to start migrating!".green());
}
