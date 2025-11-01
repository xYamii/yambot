use super::queue::{TTSQueue, TTSRequest};
use log::{error, info};
use std::path::PathBuf;
use urlencoding::encode;

const TTS_DIRECTORY: &str = "./assets/tts";
const MAX_TEXT_LENGTH: usize = 200;

pub struct TTSService {
    queue: TTSQueue,
}

impl TTSService {
    pub fn new(queue: TTSQueue) -> Self {
        // Create TTS directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(TTS_DIRECTORY) {
            error!("Failed to create TTS directory: {}", e);
        }

        Self { queue }
    }

    /// Generate TTS audio file from Google Translate API
    pub async fn generate_tts(
        &self,
        text: &str,
        language: &str,
        unique_id: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let encoded_text = encode(text);
        let url = format!(
            "https://translate.google.com/translate_tts?ie=UTF-8&q={}&tl={}&client=tw-ob",
            encoded_text, language
        );

        // Create a unique filename based on hash of text, language, and unique_id
        // This ensures duplicate messages get different files
        let hash = format!("{:x}", md5::compute(format!("{}{}{}", text, language, unique_id)));
        let file_path = PathBuf::from(TTS_DIRECTORY).join(format!("{}.mp3", hash));

        // Download the TTS audio
        let response = reqwest::get(&url).await?;

        if !response.status().is_success() {
            return Err(format!("Failed to generate TTS: HTTP {}", response.status()).into());
        }

        let bytes = response.bytes().await?;
        tokio::fs::write(&file_path, bytes).await?;

        info!(
            "Generated TTS file: {} for text: '{}' in language: {}",
            file_path.display(),
            text,
            language
        );

        Ok(file_path)
    }


    /// Split text into chunks if longer than MAX_TEXT_LENGTH
    pub fn split_text(&self, text: &str) -> Vec<String> {
        if text.len() <= MAX_TEXT_LENGTH {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for word in text.split_whitespace() {
            if current_chunk.len() + word.len() + 1 > MAX_TEXT_LENGTH {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk.clear();
                }
            }
            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(word);
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        chunks
    }

    /// Process TTS request (generate files for all chunks)
    /// Returns list of generated file paths
    pub async fn process_request(
        &self,
        request: &TTSRequest,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
        let chunks = self.split_text(&request.text);
        let mut file_paths = Vec::new();

        // Use message ID + chunk index to ensure uniqueness
        for (index, chunk) in chunks.iter().enumerate() {
            let unique_id = format!("{}-{}", request.id, index);
            let file_path = self.generate_tts(chunk, &request.language, &unique_id).await?;
            file_paths.push(file_path);
        }

        Ok(file_paths)
    }

    pub fn queue(&self) -> &TTSQueue {
        &self.queue
    }
}
