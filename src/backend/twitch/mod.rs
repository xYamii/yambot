/// Twitch EventSub WebSocket integration module
///
/// This module now re-exports the `twitchy` crate for Twitch chat integration.
///
/// # Example Usage
///
/// ```rust,no_run
/// use yambot::backend::twitch::{TwitchClient, TwitchConfig, TwitchClientEvent};
/// use tokio::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() {
///     let config = TwitchConfig::builder()
///         .channel("your_channel")
///         .tokens("access_token", "refresh_token")
///         .credentials("client_id", "client_secret")
///         .build()
///         .unwrap();
///
///     let (tx, mut rx) = mpsc::channel(100);
///     let mut client = TwitchClient::new(config);
///
///     // Connect to Twitch
///     client.connect(tx).await.unwrap();
///
///     // Listen for events
///     while let Some(event) = rx.recv().await {
///         match event {
///             TwitchClientEvent::ChatEvent(chat_event) => {
///                 // Handle chat event
///             }
///             _ => {}
///         }
///     }
/// }
/// ```

// Re-export all public types from twitchy
pub use twitchy::*;
