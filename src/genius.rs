use anyhow::Result;
use regex::Regex;
use reqwest::{Client, Method, Url};
use scraper::Html;
use serde::Deserialize;
use tracing::{debug, info, trace, warn};

const GENIUS_ENDPOINT: &str = "https://api.genius.com";

#[derive(Deserialize, Debug)]
struct SearchResults {
    response: SearchResultsResponse,
}

#[derive(Deserialize, Debug)]
struct SearchResultsResponse {
    hits: Vec<SearchResultsHit>,
}

#[derive(Deserialize, Debug)]
struct SearchResultsHit {
    #[serde(rename = "type")]
    res_type: String,
    result: SearchResultsResult,
}

#[derive(Deserialize, Debug)]
struct SearchResultsResult {
    url: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PreloadedState {
    song_page: SongPage,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SongPage {
    lyrics_data: LyricsData,
}

#[derive(Deserialize, Debug)]
struct LyricsData {
    body: LyricsDataBody,
}

#[derive(Deserialize, Debug)]
struct LyricsDataBody {
    html: String,
}

pub struct LyricsClient {
    access_token: String,
    http_client: Client,
    // lyrics_selector: Selector,
    // instrumental_selector: Selector,
}

impl LyricsClient {
    /// Creates a new Genius lyrics client,
    /// using the specified access token to authenticate requests.
    pub fn new(access_token: String) -> Self {
        let http_client = Client::new();

        Self {
            access_token,
            http_client,
        }
    }

    /// Attempts to get the lyrics for the provided artist and song title.
    pub async fn get_lyrics(&self, artist: &str, title: &str) -> Result<Option<String>> {
        let song_url = self.search(artist, title).await?;

        if let Some(song_url) = song_url {
            info!("Found song lyrics page: '{song_url}'");

            let lyrics = self.scrape_lyrics(song_url).await?;

            Ok(Some(lyrics))
        } else {
            Ok(None)
        }
    }

    /// Attempts to search Genius using the API for results matching the provided artist and song title.
    /// Returns the song URL of the first result, if any.
    async fn search(&self, artist: &str, title: &str) -> Result<Option<String>> {
        let url = Url::parse(&format!("{GENIUS_ENDPOINT}/search")).expect("Failed to parse URL");

        trace!("Querying '{url}'");

        let search_results = self
            .http_client
            .request(Method::GET, url)
            .bearer_auth(&self.access_token)
            .query(&[("q", &format!("{artist} {title}"))])
            .send()
            .await?
            .json::<SearchResults>()
            .await?;

        let res = search_results.response.hits.into_iter().find_map(|hit| {
            if hit.res_type == "song" {
                Some(hit.result.url)
            } else {
                warn!("No matches found for song");
                None
            }
        });

        Ok(res)
    }

    /// Downloads the song page and scrapes the lyrics from the response HTML.
    ///
    /// The complete lyrics do not always load into the static page, as they are hydrated by the client.
    /// Luckily the full lyrics are stored as an initial state JSON value inside a script tag,
    /// which we can rip out and parse to get the HTML, then scrape that.
    async fn scrape_lyrics(&self, song_url: String) -> Result<String> {
        let page_html = reqwest::get(song_url).await?.text().await?;
        debug!("Downloaded song page, scraping lyrics");

        // If this doesn't break...
        let regex = Regex::new("window\\.__PRELOADED_STATE__ ?= ?JSON\\.parse\\('(.*)'\\)")
            .expect("Failed to parse regex");

        let preload_data = regex
            .captures(&page_html)
            .and_then(|cap| cap.get(1))
            .map(|m| {
                m.as_str()
                    // Unescape JSON
                    .replace("\\$", "$")
                    .replace("\\\"", "\"")
                    .replace("\\\'", "\'")
                    .replace("\\\\", "\\")
            })
            .expect("Failed to get preloaded state from page. This probably means Genius has changed its page structure :(");

        let preload_data = serde_json::from_str::<PreloadedState>(preload_data.as_str())?;

        let page = Html::parse_fragment(&preload_data.song_page.lyrics_data.body.html);

        let element = page.root_element();

        Ok(element
            .text()
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string())
    }
}
