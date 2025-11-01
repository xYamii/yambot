use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTSRequest {
    pub id: String,
    pub username: String,
    pub language: String,
    pub text: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct TTSQueueItem {
    pub request: TTSRequest,
    pub file_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct TTSQueue {
    queue: Arc<Mutex<VecDeque<TTSQueueItem>>>,
    ignored_users: Arc<Mutex<Vec<String>>>,
}

impl TTSQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            ignored_users: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn add(&self, item: TTSQueueItem) {
        let mut queue = self.queue.lock().await;
        queue.push_back(item);
    }

    pub async fn pop(&self) -> Option<TTSQueueItem> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }

    pub async fn peek(&self) -> Option<TTSQueueItem> {
        let queue = self.queue.lock().await;
        queue.front().cloned()
    }

    pub async fn clear(&self) {
        let mut queue = self.queue.lock().await;
        queue.clear();
    }

    pub async fn remove(&self, id: &str) -> bool {
        let mut queue = self.queue.lock().await;
        if let Some(pos) = queue.iter().position(|item| item.request.id == id) {
            queue.remove(pos);
            true
        } else {
            false
        }
    }

    pub async fn skip_current(&self) -> Option<TTSQueueItem> {
        self.pop().await
    }

    pub async fn ignore_user(&self, username: &str) {
        let mut ignored = self.ignored_users.lock().await;
        if !ignored.contains(&username.to_string()) {
            ignored.push(username.to_string());
        }
    }

    pub async fn unignore_user(&self, username: &str) {
        let mut ignored = self.ignored_users.lock().await;
        ignored.retain(|u| u != username);
    }

    pub async fn is_user_ignored(&self, username: &str) -> bool {
        let ignored = self.ignored_users.lock().await;
        ignored.contains(&username.to_string())
    }

    pub async fn get_all(&self) -> Vec<TTSQueueItem> {
        let queue = self.queue.lock().await;
        queue.iter().cloned().collect()
    }

    pub async fn len(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }

    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.lock().await;
        queue.is_empty()
    }
}

impl Default for TTSQueue {
    fn default() -> Self {
        Self::new()
    }
}
