# mpd-lyricsd

Lyrics fetching service for MPD.

Currently capable of fetching lyrics from Genius only.

## Installation

### Cargo

```bash
cargo install mpd-lyricsd
```

[crate](https://crates.io/mpd-lyricsd)

### From source

```bash
git clone https://github.com/jakestanger/mpd-lyricsd
cd mpd-lyricsd
cargo build --release
```

## Configuration

mpd-lyricsd uses [universal-config](https://github.com/jakestanger/universal-config-rs), 
which means it supports any of JSON, YAML, TOML, and [Corn](https://github.com/jakestanger/corn).

Create a file of your preferred type at `~/.config/mpd-lyricsd/` called `config`, for example `config.corn`.

| Name                  | Type   | Default          | Description                                            |
|-----------------------|--------|------------------|--------------------------------------------------------|
| `lyrics_path`         | String | `null`           | **[Required]** Path to save lyrics on disk.            |
| `genius.access_token` | String | `null`           | **[Required]** Access token for Genius API. See below. |
| `mpd.address`         | String | `localhost:6600` | TCP or Unix socket to connect to MPD on.               |

### Example

`config.toml`:

```toml
lyrics_path = "/home/jake/Music/.lyrics"

[genius]
access_token = "<redacted>"

[mpd]
address = "media-server:6600"
```

### Genius access token

Genius requires you to provide an access token to authenticate against the API. 

To create one, you require a Genius account, and then must create an API Client registration.
Create one here: https://genius.com/api-clients/new.

You can use any app name and website URL.

Once created, generate an access token and copy it into your config.
