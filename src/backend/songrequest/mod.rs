use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct SongRequest {
    pub id: String,
    pub youtube_id: String,
    pub url: String,
    pub title: String,
    pub channel: String,
    pub duration: String,
    pub requested_by: String,
}

#[derive(Clone)]
pub struct SongRequestQueue {
    inner: Arc<Mutex<VecDeque<SongRequest>>>,
}

impl SongRequestQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn add(&self, req: SongRequest) {
        self.inner.lock().await.push_back(req);
    }

    pub async fn skip_current(&self) -> Option<SongRequest> {
        self.inner.lock().await.pop_front()
    }

    pub async fn remove_by_id(&self, id: &str) -> bool {
        let mut q = self.inner.lock().await;
        if let Some(pos) = q.iter().position(|r| r.id == id) {
            q.remove(pos);
            true
        } else {
            false
        }
    }

    pub async fn remove_last_by_user(&self, username: &str) -> bool {
        let mut q = self.inner.lock().await;
        let pos = q
            .iter()
            .enumerate()
            .filter(|(_, r)| r.requested_by.eq_ignore_ascii_case(username))
            .map(|(i, _)| i)
            .last();
        if let Some(pos) = pos {
            q.remove(pos);
            true
        } else {
            false
        }
    }

    pub async fn get_all(&self) -> Vec<SongRequest> {
        self.inner.lock().await.iter().cloned().collect()
    }
}

/// Extracts the 11-character YouTube video ID from various URL formats.
pub fn extract_video_id(url: &str) -> Option<String> {
    // youtu.be/<id>
    if let Some(rest) = url.strip_prefix("https://youtu.be/")
        .or_else(|| url.strip_prefix("http://youtu.be/"))
    {
        let id = rest.split('?').next()?.trim();
        if id.len() == 11 {
            return Some(id.to_string());
        }
    }

    // youtube.com/watch?v=<id> or music.youtube.com/watch?v=<id>
    if url.contains("youtube.com/watch") {
        for param in url.split('?').nth(1).unwrap_or("").split('&') {
            if let Some(id) = param.strip_prefix("v=") {
                let id = id.split('&').next()?.trim();
                if id.len() == 11 {
                    return Some(id.to_string());
                }
            }
        }
    }

    None
}

/// Fetches video title, channel, and formatted duration from YouTube Data API v3.
/// Returns ("title", "channel", "M:SS") or an error.
/// If `api_key` is empty, returns the raw URL as title with empty channel/duration.
pub async fn fetch_video_metadata(
    api_key: &str,
    video_id: &str,
    fallback_url: &str,
) -> Result<(String, String, String), Box<dyn std::error::Error + Send + Sync>> {
    if api_key.is_empty() {
        return Ok((fallback_url.to_string(), String::new(), String::new()));
    }

    let url = format!(
        "https://www.googleapis.com/youtube/v3/videos?part=snippet,contentDetails&id={}&key={}",
        video_id, api_key
    );

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

    let item = resp["items"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or("No video found")?;

    let title = item["snippet"]["title"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();
    let channel = item["snippet"]["channelTitle"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let iso_duration = item["contentDetails"]["duration"]
        .as_str()
        .unwrap_or("PT0S");
    let duration = parse_iso8601_duration(iso_duration);

    Ok((title, channel, duration))
}

/// Converts ISO 8601 duration (e.g. "PT3M42S") to "3:42".
fn parse_iso8601_duration(iso: &str) -> String {
    let s = iso.trim_start_matches("PT");
    let hours = extract_component(s, 'H');
    let minutes = extract_component(s, 'M');
    let seconds = extract_component(s, 'S');

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

fn extract_component(s: &str, unit: char) -> u64 {
    s.find(unit).and_then(|end| {
        let start = s[..end].rfind(|c: char| !c.is_ascii_digit()).map(|i| i + 1).unwrap_or(0);
        s[start..end].parse().ok()
    }).unwrap_or(0)
}
