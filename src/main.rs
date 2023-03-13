mod genius;

use crate::genius::LyricsClient;
use anyhow::Result;
use mpd_client::client::{ConnectionEvent, Subsystem};
use mpd_client::{commands, Client};
use serde::Deserialize;
use std::path::Path;
use tokio::fs;
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct Config {
    // TODO: Implement
    #[serde(default = "default_lyrics_providers")]
    providers: Vec<LyricsProvider>,
    lyrics_path: String,
    genius: GeniusConfig,
    #[serde(default)]
    mpd: MpdConfig,
}

#[derive(Deserialize)]
enum LyricsProvider {
    Genius,
}

#[derive(Deserialize)]
struct GeniusConfig {
    access_token: String,
}

#[derive(Deserialize)]
struct MpdConfig {
    address: String,
}

impl Default for MpdConfig {
    fn default() -> Self {
        Self {
            address: String::from("localhost:6600"),
        }
    }
}

fn default_lyrics_providers() -> Vec<LyricsProvider> {
    vec![LyricsProvider::Genius]
}

#[tokio::main]
async fn main() -> Result<()> {
    const DEFAULT_LOG: &str = "info";

    let filter_layer =
        EnvFilter::try_from_env("LYRICSD_LOG").or_else(|_| EnvFilter::try_new(DEFAULT_LOG))?;

    tracing_subscriber::fmt()
        .with_env_filter(filter_layer)
        .init();

    info!("mpd-lyricsd version {}", VERSION);
    info!("Starting application");

    let config = universal_config::ConfigLoader::new("mpd-lyricsd").find_and_load::<Config>()?;

    // TODO: Improve MPD connection code - want to be able to connect to multiple servers like we can in mpd-discord-rpc.
    let connection = TcpStream::connect(config.mpd.address).await?;
    let (mpd_client, mut events) = Client::connect(connection).await?;

    let genius = LyricsClient::new(config.genius.access_token);

    fs::create_dir_all(&config.lyrics_path).await?;

    let mut prev_song = None;
    while let Some(event) = events.next().await {
        if matches!(
            event,
            ConnectionEvent::SubsystemChange(Subsystem::Queue | Subsystem::Player)
        ) {
            debug!("Detected song change");

            let song = mpd_client.command(commands::CurrentSong).await?;

            let metadata = song.map(|s| s.song).map(|song| {
                (
                    song.artists().first().map(ToString::to_string),
                    song.title().map(ToString::to_string),
                )
            });

            match metadata {
                Some((Some(artist), Some(title))) => {
                    let cache_val = Some((artist.to_string(), title.to_string()));

                    if prev_song == cache_val {
                        debug!("New song same name as previous - skipping");
                    } else {
                        prev_song = cache_val;

                        info!("New song detected - '{artist} - {title}'");
                        process_change(&genius, &config.lyrics_path, &artist, &title).await?;
                    }
                }
                Some(_) => warn!("New song missing title/artist metadata - skipping"),
                _ => {}
            }
        }
    }

    Ok(())
}

async fn process_change(
    genius: &LyricsClient,
    lyrics_path: &str,
    artist: &str,
    title: &str,
) -> Result<()> {
    let song_path = Path::new(lyrics_path).join(format!("{artist} - {title}.txt"));

    if matches!(fs::try_exists(&song_path).await, Ok(true)) {
        info!("Lyrics file for '{artist} - {title}' already exists - skipping");
    } else {
        let lyrics = genius.get_lyrics(artist, title).await;

        match lyrics {
            Ok(Some(lyrics)) => {
                fs::write(&song_path, lyrics).await?;
                info!("Saved lyrics to '{}'", &song_path.display());
            }
            Ok(None) => {
                warn!("Unable to find lyrics for '{artist} - {title}'");
            }
            Err(err) => error!("{err:?}"),
        }
    };

    Ok(())
}
