# Yambot

Yambot is a chatbot for Twitch IRC written in Rust.

## Features

todo

## Configuration

Update `\src\backend\twitch\auth.rs` and set client_id and client_secret in order to make bot connect to twitch chat.

## Usage

todo

## Required scopes
In order for bot to handle everything you need scopes:
- user:read:chat
- channel:bot
- user:bot
- channel:moderate
- user:write:chat

You can use to https://yamii.bieda.it/ to generate access token.

*Building app yourself requires you to generate access token with client_id set in auth.rs*

## Song Request — YouTube API Key

Song requests use the YouTube Data API v3 to fetch song titles and durations. To enable it:

1. Go to [Google Cloud Console](https://console.cloud.google.com/) and create a project (or select an existing one).
2. Navigate to **APIs & Services → Library**, search for **YouTube Data API v3**, and click **Enable**.
3. Go to **APIs & Services → Credentials**, click **Create Credentials → API key**, and copy the generated key.
4. Paste the key in the bot's **Settings → Song Request → YouTube API key** field and click **Save Song Request Settings**, or add it directly to `config.toml`:

```toml
[song_request]
enabled = true
youtube_api_key = "AIza..."
```

The free tier allows 10,000 requests per day, which is more than enough for normal stream usage (each `!sr` uses 1 unit).

## Contributing

If you have any ideas, suggestions, or bug reports, please open an issue or submit a pull request on the [GitHub repository](https://github.com/xyamii/yambot).


