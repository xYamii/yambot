use super::queue::{TTSAudioChunk, TTSQueue, TTSRequest};
use log::info;
use urlencoding::encode;
use std::time::Duration;

const MAX_TEXT_LENGTH: usize = 200;

pub struct TTSService {
    queue: TTSQueue,
    client: reqwest::Client,
}

impl TTSService {
    pub fn new(queue: TTSQueue) -> Self {
        // Create HTTP client with connection pooling and timeout
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(5)
            .build()
            .expect("Failed to create HTTP client");

        Self { queue, client }
    }

    /// Fetch TTS audio data as bytes from Google Translate API
    pub async fn fetch_tts_audio(
        &self,
        text: &str,
        language: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Validate language code - only allow alphanumeric and hyphens (e.g., "en", "en-US")
        if !language.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(format!("Invalid language code: {}", language).into());
        }

        // Limit language code length
        if language.len() > 10 {
            return Err("Language code too long".into());
        }

        let encoded_text = encode(text);
        let url = format!(
            "https://translate.google.com/translate_tts?ie=UTF-8&q={}&tl={}&client=tw-ob",
            encoded_text, language
        );

        // Download the TTS audio using the pooled client
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to generate TTS: HTTP {}", response.status()).into());
        }

        let bytes = response.bytes().await?;

        info!(
            "Fetched TTS audio for text: '{}' in language: {} ({} bytes)",
            text,
            language,
            bytes.len()
        );

        Ok(bytes.to_vec())
    }


    /// Split text into chunks if longer than MAX_TEXT_LENGTH
    pub fn split_text(&self, text: &str) -> Vec<String> {
        if text.len() <= MAX_TEXT_LENGTH {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for word in text.split_whitespace() {
            // Handle single words longer than MAX_TEXT_LENGTH
            if word.len() > MAX_TEXT_LENGTH {
                // Flush current chunk if not empty
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk.clear();
                }

                // Split long word into character chunks
                let mut remaining = word;
                while !remaining.is_empty() {
                    let split_at = remaining.char_indices()
                        .nth(MAX_TEXT_LENGTH)
                        .map(|(idx, _)| idx)
                        .unwrap_or(remaining.len());
                    chunks.push(remaining[..split_at].to_string());
                    remaining = &remaining[split_at..];
                }
                continue;
            }

            // Check if adding this word would exceed limit
            if current_chunk.len() + word.len() + 1 > MAX_TEXT_LENGTH {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk.clear();
                }
            }

            // Add word to current chunk
            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(word);
        }

        // Push final chunk if not empty
        if !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        chunks
    }

    /// Process TTS request (fetch audio for all chunks)
    /// Returns list of audio chunks
    pub async fn process_request(
        &self,
        request: &TTSRequest,
    ) -> Result<Vec<TTSAudioChunk>, Box<dyn std::error::Error + Send + Sync>> {
        let chunks = self.split_text(&request.text);
        let mut audio_chunks = Vec::new();

        for chunk in chunks.iter() {
            let audio_data = self.fetch_tts_audio(chunk, &request.language).await?;
            audio_chunks.push(TTSAudioChunk { audio_data });
        }

        Ok(audio_chunks)
    }

    pub fn queue(&self) -> &TTSQueue {
        &self.queue
    }
}
