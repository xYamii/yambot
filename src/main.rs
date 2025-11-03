use crate::backend::commands::{CommandExecutor, CommandParser, CommandRegistry, CommandResult};
use crate::backend::tts::{
    LanguageConfig, TTSAudioChunk, TTSQueue, TTSQueueItem, TTSRequest, TTSService,
};
use crate::backend::twitch::{
    ChatMessageEvent, TwitchClient, TwitchClientEvent, TwitchConfig, TwitchEvent,
};
use backend::config::AppConfig;
use eframe::egui::{self};
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use ui::{BackendToFrontendMessage, FrontendToBackendMessage};

pub mod backend;
pub mod ui;
use log::{error, info};

// Audio playback request for SFX system
#[derive(Debug, Clone)]
struct AudioPlaybackRequest {
    file_path: String,
    volume: f32,
    is_full_path: bool,
}

// Channel for sending audio playback requests
// Using std::sync::mpsc::Sender wrapped for compatibility with async code
#[derive(Clone)]
struct AudioPlaybackSender(std::sync::mpsc::Sender<AudioPlaybackRequest>);

impl AudioPlaybackSender {
    fn send_sound(
        &self,
        sound: String,
        volume: f32,
    ) -> Result<(), std::sync::mpsc::SendError<AudioPlaybackRequest>> {
        self.0.send(AudioPlaybackRequest {
            file_path: sound,
            volume,
            is_full_path: false,
        })
    }
}

const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub message_id: String,
    pub message_text: String,
    pub badges: Vec<String>,
    pub username: String,
    pub user_id: String,
    pub color: String,
}

impl From<ChatMessageEvent> for ChatMessage {
    fn from(msg: ChatMessageEvent) -> Self {
        let badges = msg
            .badges
            .into_iter()
            .map(|badge| format!("{}-{}", badge.set_id, badge.id))
            .collect();

        ChatMessage {
            message_id: msg.message_id,
            message_text: msg.message.text,
            badges,
            username: msg.chatter_user_login,
            user_id: msg.chatter_user_id,
            color: msg.color,
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let (backend_tx, frontend_rx) = tokio::sync::mpsc::channel(100);
    let (frontend_tx, backend_rx) = tokio::sync::mpsc::channel(100);
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_resizable(false),
        ..Default::default()
    };
    let config = backend::config::load_config();
    let command_registry = backend::config::load_commands();

    // Initialize SoundsManager to start file watching
    // Spawn it in a task to keep it alive for the entire application lifetime
    let backend_tx_for_sounds = backend_tx.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let _sounds_manager = backend::sfx::SoundsManager::new(backend_tx_for_sounds)
                .await
                .expect("Failed to initialize SoundsManager");

            // Keep the watcher alive forever
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });
    });

    // Wrap command registry in Arc<RwLock> for sharing across tasks
    let shared_registry = Arc::new(RwLock::new(command_registry));

    // Create audio playback channel and spawn dedicated audio task in a blocking thread
    // This solves the OutputStream Send issue on macOS by creating OutputStream in a dedicated thread
    let (audio_tx, audio_rx) = std::sync::mpsc::channel::<AudioPlaybackRequest>();
    let audio_tx = AudioPlaybackSender(audio_tx);
    std::thread::spawn(move || {
        // Create the OutputStream inside the thread to avoid Send issues on macOS
        let stream = rodio::OutputStreamBuilder::open_default_stream()
            .expect("Failed to open default audio stream");
        audio_playback_task(audio_rx, stream);
    });

    // Initialize TTS system
    let tts_queue = TTSQueue::new();
    let tts_service = Arc::new(TTSService::new(tts_queue.clone()));
    let language_config = Arc::new(RwLock::new(backend::tts::load_language_config()));

    // Start TTS player task using tokio
    let tts_queue_for_player = tts_queue.clone();
    let backend_tx_for_player = backend_tx.clone();
    tokio::spawn(async move {
        tts_player_task(tts_queue_for_player, backend_tx_for_player).await;
    });

    let registry_clone = shared_registry.clone();
    let audio_tx_clone = audio_tx.clone();
    let tts_queue_clone = tts_queue.clone();
    let tts_service_clone = tts_service.clone();
    let language_config_clone = language_config.clone();
    tokio::spawn(async move {
        handle_frontend_to_backend_messages(
            backend_rx,
            backend_tx.clone(),
            audio_tx_clone,
            registry_clone,
            tts_queue_clone,
            tts_service_clone,
            language_config_clone,
        )
        .await;
    });
    info!("Starting chatbot");

    // Get initial commands for UI
    let commands = {
        let registry = shared_registry.read().await;
        registry.list().iter().map(|c| (*c).clone()).collect()
    };

    // Get TTS languages for UI
    let tts_languages = {
        let lang_cfg = language_config.read().await;
        lang_cfg
            .get_all_languages()
            .iter()
            .map(|l| (*l).clone())
            .collect()
    };

    let _ = eframe::run_native(
        "Yambot",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx.set_style(egui::Style {
                visuals: egui::Visuals::dark(),
                ..egui::Style::default()
            });
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // read values from env or other config file that will be updated later on
            Ok(Box::new(ui::Chatbot::new(
                config.chatbot,
                frontend_tx,
                frontend_rx,
                config.sfx,
                config.tts,
                tts_languages,
                commands,
            )))
        }),
    )
    .map_err(|e| error!("Error: {:?}", e));
}

async fn handle_twitch_messages(
    config: TwitchConfig,
    backend_tx: tokio::sync::mpsc::Sender<ui::BackendToFrontendMessage>,
    audio_tx: AudioPlaybackSender,
    command_registry: Arc<RwLock<CommandRegistry>>,
    tts_queue: TTSQueue,
    tts_service: Arc<TTSService>,
    language_config: Arc<RwLock<LanguageConfig>>,
    welcome_message: Option<String>,
) {
    // TODO: add messages to local db
    let mut messages: Vec<ChatMessage> = Vec::new();
    let command_parser = CommandParser::with_default_prefix();

    // Create event channel
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    // Create and connect Twitch client
    let mut client = TwitchClient::new(config);

    match client.connect(tx).await {
        Ok(_) => {
            let _ = backend_tx
                .send(ui::BackendToFrontendMessage::ConnectionSuccess(
                    "Connected".to_string(),
                ))
                .await;
            let _ = backend_tx
                .send(ui::BackendToFrontendMessage::CreateLog(
                    ui::LogLevel::INFO,
                    "Successfully connected to Twitch chat".to_string(),
                ))
                .await;

            // Send welcome message if configured
            if let Some(ref msg) = welcome_message {
                if !msg.trim().is_empty() {
                    log::info!("Attempting to send welcome message: {}", msg);

                    // Wait a moment for subscriptions to settle
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                    match client.send_message(msg).await {
                        Ok(_) => {
                            log::info!("Welcome message sent successfully");
                            let _ = backend_tx
                                .send(ui::BackendToFrontendMessage::CreateLog(
                                    ui::LogLevel::INFO,
                                    format!("✓ Sent welcome message: {}", msg),
                                ))
                                .await;
                        }
                        Err(e) => {
                            log::error!("Failed to send welcome message: {}", e);
                            let error_str = e.to_string();

                            let user_msg = if error_str.contains("403")
                                || error_str.contains("Forbidden")
                            {
                                format!("❌ Cannot send welcome message - Missing OAuth scope 'user:write:chat'. Please re-authorize with write permissions.")
                            } else {
                                format!("❌ Failed to send welcome message: {}", e)
                            };

                            let _ = backend_tx
                                .send(ui::BackendToFrontendMessage::CreateLog(
                                    ui::LogLevel::ERROR,
                                    user_msg,
                                ))
                                .await;
                        }
                    }
                }
            }
        }
        Err(e) => {
            let _ = backend_tx
                .send(ui::BackendToFrontendMessage::ConnectionFailure(
                    "Connection Failed".to_string(),
                ))
                .await;
            let _ = backend_tx
                .send(ui::BackendToFrontendMessage::CreateLog(
                    ui::LogLevel::ERROR,
                    format!("Failed to connect: {}", e),
                ))
                .await;
            return;
        }
    }

    // Handle incoming events
    while let Some(event) = rx.recv().await {
        match event {
            TwitchClientEvent::Connected => {
                let _ = backend_tx
                    .send(ui::BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::INFO,
                        "EventSub ready - listening for chat events".to_string(),
                    ))
                    .await;
            }

            TwitchClientEvent::ChatEvent(chat_event) => match chat_event {
                TwitchEvent::ChatMessage(msg) => {
                    let chat_message: ChatMessage = msg.clone().into();

                    // Check if message is a TTS command (e.g., !en hello, !pl czesc)
                    let message_text = msg.message.text.trim().to_lowercase();
                    if message_text.starts_with('!') && message_text.len() > 1 {
                        let parts: Vec<&str> = message_text.splitn(2, ' ').collect();
                        if parts.len() == 2 {
                            let potential_lang_code = &parts[0][1..]; // Remove the '!' prefix
                            let tts_text = parts[1];

                            // Check if this is a valid language code
                            let lang_config = language_config.read().await;
                            if let Some(language) = lang_config.get_language(potential_lang_code) {
                                if language.enabled {
                                    // Check TTS config and permissions
                                    let config = backend::config::load_config();
                                    if config.tts.enabled {
                                        // Check user permissions
                                        let has_permission = msg.badges.iter().any(|badge| {
                                            (badge.set_id == "subscriber"
                                                || badge.set_id == "founder")
                                                && config.tts.permited_roles.subs
                                                || badge.set_id == "vip"
                                                    && config.tts.permited_roles.vips
                                                || badge.set_id == "moderator"
                                                    && config.tts.permited_roles.mods
                                                || badge.set_id == "broadcaster"
                                        });

                                        if !has_permission {
                                            continue;
                                        }
                                        if tts_queue.is_user_ignored(&msg.chatter_user_login).await
                                        {
                                            continue;
                                        }

                                        let tts_request = TTSRequest {
                                            id: msg.message_id.clone(),
                                            username: msg.chatter_user_login.clone(),
                                            language: potential_lang_code.to_string(),
                                            text: tts_text.to_string(),
                                            timestamp: chrono::Utc::now(),
                                        };

                                        // Generate TTS files asynchronously
                                        let tts_service_clone = tts_service.clone();
                                        let tts_queue_clone = tts_queue.clone();
                                        let backend_tx_clone = backend_tx.clone();
                                        let request_clone = tts_request.clone();

                                        tokio::spawn(async move {
                                            // Split text into chunks
                                            let text_chunks =
                                                tts_service_clone.split_text(&request_clone.text);
                                            let chunk_count = text_chunks.len();

                                            // Process each chunk as a separate queue item
                                            for (chunk_index, text_chunk) in
                                                text_chunks.into_iter().enumerate()
                                            {
                                                // Create unique ID for this chunk
                                                let chunk_id = if chunk_count > 1 {
                                                    format!("{}-{}", request_clone.id, chunk_index)
                                                } else {
                                                    request_clone.id.clone()
                                                };

                                                // Fetch audio for this chunk
                                                match tts_service_clone
                                                    .fetch_tts_audio(
                                                        &text_chunk,
                                                        &request_clone.language,
                                                    )
                                                    .await
                                                {
                                                    Ok(audio_data) => {
                                                        let chunk_request = TTSRequest {
                                                            id: chunk_id,
                                                            username: request_clone
                                                                .username
                                                                .clone(),
                                                            language: request_clone
                                                                .language
                                                                .clone(),
                                                            text: text_chunk,
                                                            timestamp: request_clone.timestamp,
                                                        };

                                                        let queue_item = TTSQueueItem {
                                                            request: chunk_request,
                                                            audio_chunks: vec![TTSAudioChunk {
                                                                audio_data,
                                                            }],
                                                        };

                                                        tts_queue_clone.add(queue_item).await;

                                                        // Send updated queue to frontend (including currently playing)
                                                        let queue_items = tts_queue_clone
                                                            .get_all_with_current()
                                                            .await;
                                                        let ui_queue: Vec<ui::TTSQueueItemUI> =
                                                            queue_items
                                                                .into_iter()
                                                                .map(|item| ui::TTSQueueItemUI {
                                                                    id: item.request.id,
                                                                    username: item.request.username,
                                                                    text: item.request.text,
                                                                    language: item.request.language,
                                                                })
                                                                .collect();
                                                        let _ = backend_tx_clone
                                                            .send(BackendToFrontendMessage::TTSQueueUpdated(ui_queue))
                                                            .await;
                                                    }
                                                    Err(e) => {
                                                        error!("Failed to fetch TTS audio for chunk {}/{}: {}", chunk_index + 1, chunk_count, e);
                                                        let _ = backend_tx_clone
                                                            .send(BackendToFrontendMessage::CreateLog(
                                                                ui::LogLevel::ERROR,
                                                                format!("Failed to generate TTS chunk: {}", e),
                                                            ))
                                                            .await;
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }
                                // If it's a valid language code, don't process as regular command
                                messages.push(chat_message);
                                continue;
                            }
                        }
                    }

                    // Check if message is a command
                    if let Some(context) = command_parser.parse(msg.clone()) {
                        // Lock the registry and execute command
                        let result = {
                            let mut registry = command_registry.write().await;
                            let mut executor = CommandExecutor::new(registry.clone());
                            let result = executor.execute(&context);

                            // Update cooldowns in the shared registry
                            *registry = executor.registry().clone();
                            result
                        };

                        match result {
                            CommandResult::Success(Some(action)) => {
                                // Parse the action and handle it
                                if let Some(send_msg) = action.strip_prefix("send:") {
                                    if let Err(e) = client.send_message(send_msg).await {
                                        let _ = backend_tx
                                            .send(BackendToFrontendMessage::CreateLog(
                                                ui::LogLevel::ERROR,
                                                format!("Failed to send message: {}", e),
                                            ))
                                            .await;
                                    }
                                } else if let Some(reply_parts) = action.strip_prefix("reply:") {
                                    let parts: Vec<&str> = reply_parts.splitn(2, ':').collect();
                                    if parts.len() == 2 {
                                        let message_id = parts[0];
                                        let reply_msg = parts[1];
                                        if let Err(e) =
                                            client.reply_to_message(reply_msg, message_id).await
                                        {
                                            error!("Failed to reply: {}", e);
                                            let _ = backend_tx
                                                .send(BackendToFrontendMessage::CreateLog(
                                                    ui::LogLevel::ERROR,
                                                    format!("Failed to reply: {}", e),
                                                ))
                                                .await;
                                        }
                                    }
                                }
                            }
                            CommandResult::Success(None) => {}
                            CommandResult::Error(e) => {
                                let _ = backend_tx
                                    .send(BackendToFrontendMessage::CreateLog(
                                        ui::LogLevel::ERROR,
                                        format!("Command error: {}", e),
                                    ))
                                    .await;
                            }
                            CommandResult::NotFound => {
                                // Check if there's a sound file with this name
                                let sound_format = backend::sfx::Soundlist::get_format();
                                let sound_path = format!(
                                    "./assets/sounds/{}.{}",
                                    context.command_name, sound_format
                                );

                                if std::path::Path::new(&sound_path).exists() {
                                    // Check if user has permission to play sounds
                                    let config = backend::config::load_config();
                                    let has_permission = context.badges().iter().any(|badge| {
                                        (badge.set_id == "subscriber" || badge.set_id == "founder")
                                            && config.sfx.permited_roles.subs
                                            || badge.set_id == "vip"
                                                && config.sfx.permited_roles.vips
                                            || badge.set_id == "moderator"
                                                && config.sfx.permited_roles.mods
                                            || badge.set_id == "broadcaster"
                                    });

                                    if has_permission && config.sfx.enabled {
                                        // Play the sound with volume from sfx config
                                        let sound_file =
                                            format!("{}.{}", context.command_name, sound_format);
                                        let _ = audio_tx
                                            .send_sound(sound_file, config.sfx.volume as f32);
                                    }
                                }
                            }
                            CommandResult::PermissionDenied => {
                                let _ = backend_tx
                                    .send(BackendToFrontendMessage::CreateLog(
                                        ui::LogLevel::WARN,
                                        format!(
                                            "User {} tried to use command !{} without permission",
                                            context.username(),
                                            context.command_name
                                        ),
                                    ))
                                    .await;
                            }
                            CommandResult::OnCooldown(_remaining) => {}
                        }
                    }

                    messages.push(chat_message);
                }

                TwitchEvent::MessageDelete(delete) => {
                    log::info!(
                        "Message {} from {} was deleted",
                        delete.message_id,
                        delete.target_user_name
                    );
                }

                TwitchEvent::ClearUserMessages(clear) => {
                    log::info!(
                        "Messages from {} were cleared (ban/timeout)",
                        clear.target_user_name
                    );
                }

                TwitchEvent::ChatClear(clear) => {
                    log::info!(
                        "Chat was cleared in {}'s channel",
                        clear.broadcaster_user_name
                    );
                }

                TwitchEvent::ChatSettingsUpdate(settings) => {
                    log::info!(
                        "Chat settings updated: slow_mode={}, sub_only={}",
                        settings.slow_mode,
                        settings.subscriber_mode
                    );
                }

                TwitchEvent::ChannelBan(ban) => {
                    let ban_type = if ban.is_permanent {
                        "permanently banned"
                    } else {
                        "timed out"
                    };
                    let duration_info = if let Some(ref ends_at) = ban.ends_at {
                        format!(" (until {})", ends_at)
                    } else {
                        String::new()
                    };

                    log::info!(
                        "🔨 {} was {} by {}: {}{}",
                        ban.user_name,
                        ban_type,
                        ban.moderator_user_name,
                        ban.reason,
                        duration_info
                    );

                    let _ = backend_tx
                        .send(BackendToFrontendMessage::CreateLog(
                            ui::LogLevel::WARN,
                            format!(
                                "{} was {} by {}: {}{}",
                                ban.user_name,
                                ban_type,
                                ban.moderator_user_name,
                                ban.reason,
                                duration_info
                            ),
                        ))
                        .await;
                }

                TwitchEvent::ChannelUnban(unban) => {
                    log::info!(
                        "✅ {} was unbanned by {}",
                        unban.user_name,
                        unban.moderator_user_name
                    );

                    let _ = backend_tx
                        .send(BackendToFrontendMessage::CreateLog(
                            ui::LogLevel::INFO,
                            format!(
                                "{} was unbanned by {}",
                                unban.user_name, unban.moderator_user_name
                            ),
                        ))
                        .await;
                }
            },

            TwitchClientEvent::TokensRefreshed(access_token, refresh_token) => {
                // Load current config
                let mut current_config = backend::config::load_config();

                // Update tokens
                current_config.chatbot.auth_token = access_token;
                current_config.chatbot.refresh_token = refresh_token;

                // Save updated config
                backend::config::save_config(&current_config);
            }

            TwitchClientEvent::Disconnected => {
                let _ = backend_tx
                    .send(ui::BackendToFrontendMessage::ConnectionFailure(
                        "Disconnected".to_string(),
                    ))
                    .await;
                let _ = backend_tx
                    .send(ui::BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::ERROR,
                        "Disconnected from Twitch".to_string(),
                    ))
                    .await;
                break;
            }

            TwitchClientEvent::Warning(w) => {
                let _ = backend_tx
                    .send(ui::BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::WARN,
                        w,
                    ))
                    .await;
            }

            TwitchClientEvent::Error(e) => {
                let _ = backend_tx
                    .send(ui::BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::ERROR,
                        format!("Twitch error: {}", e),
                    ))
                    .await;
            }
        }
    }
}
async fn handle_frontend_to_backend_messages(
    mut backend_rx: tokio::sync::mpsc::Receiver<FrontendToBackendMessage>,
    backend_tx: tokio::sync::mpsc::Sender<BackendToFrontendMessage>,
    audio_tx: AudioPlaybackSender,
    command_registry: Arc<RwLock<CommandRegistry>>,
    tts_queue: TTSQueue,
    tts_service: Arc<TTSService>,
    language_config: Arc<RwLock<LanguageConfig>>,
) {
    // Store the handle to the twitch message handler task so we can abort it on disconnect
    let mut twitch_task_handle: Option<tokio::task::JoinHandle<()>> = None;
    while let Some(message) = backend_rx.recv().await {
        match message {
            FrontendToBackendMessage::AddTTSLang(lang_code) => {
                let mut config = language_config.write().await;
                config.enable_language(&lang_code);
                if let Err(e) = backend::tts::save_language_config(&config) {
                    let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::ERROR,
                        format!("Failed to save language config: {}", e),
                    ));
                } else {
                    let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::INFO,
                        format!("Language {} enabled", lang_code),
                    ));
                    // Send updated language list to frontend
                    let updated_langs = config
                        .get_all_languages()
                        .iter()
                        .map(|l| (*l).clone())
                        .collect();
                    let _ = backend_tx
                        .try_send(BackendToFrontendMessage::TTSLangListUpdated(updated_langs));
                }
            }
            FrontendToBackendMessage::RemoveTTSLang(lang_code) => {
                let mut config = language_config.write().await;
                config.disable_language(&lang_code);
                if let Err(e) = backend::tts::save_language_config(&config) {
                    let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::ERROR,
                        format!("Failed to save language config: {}", e),
                    ));
                } else {
                    let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::INFO,
                        format!("Language {} disabled", lang_code),
                    ));
                    // Send updated language list to frontend
                    let updated_langs = config
                        .get_all_languages()
                        .iter()
                        .map(|l| (*l).clone())
                        .collect();
                    let _ = backend_tx
                        .try_send(BackendToFrontendMessage::TTSLangListUpdated(updated_langs));
                }
            }
            FrontendToBackendMessage::UpdateTTSConfig(config) => {
                let current_config: AppConfig = backend::config::load_config();
                backend::config::save_config(
                    &(AppConfig {
                        chatbot: current_config.chatbot,
                        sfx: current_config.sfx,
                        tts: config,
                    }),
                );
                let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                    ui::LogLevel::INFO,
                    "TTS config updated".to_string(),
                ));
            }
            FrontendToBackendMessage::UpdateSfxConfig(config) => {
                let current_config: AppConfig = backend::config::load_config();
                backend::config::save_config(
                    &(AppConfig {
                        chatbot: current_config.chatbot,
                        sfx: config,
                        tts: current_config.tts,
                    }),
                );
                let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                    ui::LogLevel::INFO,
                    "SFX config updated".to_string(),
                ));
            }
            FrontendToBackendMessage::UpdateConfig(config) => {
                let current_config: AppConfig = backend::config::load_config();
                backend::config::save_config(
                    &(AppConfig {
                        chatbot: config,
                        sfx: current_config.sfx,
                        tts: current_config.tts,
                    }),
                );
                let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                    ui::LogLevel::INFO,
                    "Chatbot config updated".to_string(),
                ));
            }
            FrontendToBackendMessage::ConnectToChat(_channel_name) => {
                // Abort any existing connection first
                if let Some(handle) = twitch_task_handle.take() {
                    handle.abort();
                    let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::INFO,
                        "Disconnecting previous session...".to_string(),
                    ));
                }

                // Load config to get auth_token and client_id
                let config = backend::config::load_config();
                let twitch_config = TwitchConfig {
                    channel_name: config.chatbot.channel_name.clone(),
                    auth_token: config.chatbot.auth_token.clone(),
                    refresh_token: config.chatbot.refresh_token.clone(),
                };

                // Get welcome message if configured
                let welcome_message = if config.chatbot.welcome_message.trim().is_empty() {
                    None
                } else {
                    Some(config.chatbot.welcome_message.clone())
                };

                let backend_tx_clone = backend_tx.clone();
                let audio_tx_clone = audio_tx.clone();
                let registry_clone = command_registry.clone();
                let tts_queue_clone = tts_queue.clone();
                let tts_service_clone = tts_service.clone();
                let language_config_clone = language_config.clone();

                // Spawn the twitch handler task and store the handle
                let handle = tokio::spawn(async move {
                    handle_twitch_messages(
                        twitch_config,
                        backend_tx_clone,
                        audio_tx_clone,
                        registry_clone,
                        tts_queue_clone,
                        tts_service_clone,
                        language_config_clone,
                        welcome_message,
                    )
                    .await;
                });
                twitch_task_handle = Some(handle);

                let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                    ui::LogLevel::INFO,
                    "Connecting to Twitch...".to_string(),
                ));
            }
            FrontendToBackendMessage::AddCommand(command) => {
                {
                    let mut registry = command_registry.write().await;
                    registry.register(command);
                    backend::config::save_commands(&registry);
                }
                let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                    ui::LogLevel::INFO,
                    "Command added".to_string(),
                ));
                let _ = backend_tx.try_send(BackendToFrontendMessage::CommandsUpdated);
            }
            FrontendToBackendMessage::RemoveCommand(trigger) => {
                {
                    let mut registry = command_registry.write().await;
                    registry.unregister(&trigger);
                    backend::config::save_commands(&registry);
                }
                let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                    ui::LogLevel::INFO,
                    format!("Command '{}' removed", trigger),
                ));
                let _ = backend_tx.try_send(BackendToFrontendMessage::CommandsUpdated);
            }
            FrontendToBackendMessage::UpdateCommand(command) => {
                {
                    let mut registry = command_registry.write().await;
                    registry.register(command);
                    backend::config::save_commands(&registry);
                }
                let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                    ui::LogLevel::INFO,
                    "Command updated".to_string(),
                ));
                let _ = backend_tx.try_send(BackendToFrontendMessage::CommandsUpdated);
            }
            FrontendToBackendMessage::ToggleCommand(trigger, enabled) => {
                let mut registry = command_registry.write().await;
                if let Some(cmd) = registry.get_mut(&trigger) {
                    cmd.enabled = enabled;
                    backend::config::save_commands(&registry);
                    let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::INFO,
                        format!(
                            "Command '{}' {}",
                            trigger,
                            if enabled { "enabled" } else { "disabled" }
                        ),
                    ));
                }
            }
            FrontendToBackendMessage::GetTTSQueue => {
                // Get all items from queue (including currently playing) and send to frontend
                let queue_items = tts_queue.get_all_with_current().await;
                let ui_queue: Vec<ui::TTSQueueItemUI> = queue_items
                    .into_iter()
                    .map(|item| ui::TTSQueueItemUI {
                        id: item.request.id,
                        username: item.request.username,
                        text: item.request.text,
                        language: item.request.language,
                    })
                    .collect();
                let _ = backend_tx.try_send(BackendToFrontendMessage::TTSQueueUpdated(ui_queue));
            }
            FrontendToBackendMessage::SkipTTSMessage(message_id) => {
                // Check if it's the currently playing item
                let is_current = if let Some(current) = tts_queue.get_currently_playing().await {
                    current.request.id == message_id
                } else {
                    false
                };

                if is_current {
                    // Skip currently playing
                    tts_queue.skip_current().await;
                }

                // Send updated queue
                let queue_items = tts_queue.get_all_with_current().await;
                let ui_queue: Vec<ui::TTSQueueItemUI> = queue_items
                    .into_iter()
                    .map(|item| ui::TTSQueueItemUI {
                        id: item.request.id,
                        username: item.request.username,
                        text: item.request.text,
                        language: item.request.language,
                    })
                    .collect();
                let _ = backend_tx.try_send(BackendToFrontendMessage::TTSQueueUpdated(ui_queue));
            }
            FrontendToBackendMessage::SkipCurrentTTS => {
                tts_queue.skip_current().await;

                // Send updated queue
                let queue_items = tts_queue.get_all_with_current().await;
                let ui_queue: Vec<ui::TTSQueueItemUI> = queue_items
                    .into_iter()
                    .map(|item| ui::TTSQueueItemUI {
                        id: item.request.id,
                        username: item.request.username,
                        text: item.request.text,
                        language: item.request.language,
                    })
                    .collect();
                let _ = backend_tx.try_send(BackendToFrontendMessage::TTSQueueUpdated(ui_queue));
            }
            FrontendToBackendMessage::DisconnectFromChat(_channel_name) => {
                // Abort the twitch message handler task if it's running
                if let Some(handle) = twitch_task_handle.take() {
                    handle.abort();
                    let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::INFO,
                        "Disconnected from Twitch".to_string(),
                    ));
                } else {
                    let _ = backend_tx.try_send(BackendToFrontendMessage::CreateLog(
                        ui::LogLevel::WARN,
                        "Not connected to Twitch".to_string(),
                    ));
                }
            }
        }
    }
}

// Dedicated audio playback task that owns the OutputStream
// This solves the Send issue on macOS by keeping OutputStream in a single blocking thread
// Handles both sound effects and TTS audio files
fn audio_playback_task(rx: std::sync::mpsc::Receiver<AudioPlaybackRequest>, stream: OutputStream) {
    while let Ok(request) = rx.recv() {
        let audio_path = if request.is_full_path {
            request.file_path
        } else {
            "./assets/sounds/".to_string() + &request.file_path
        };

        if let Ok(file) = File::open(Path::new(&audio_path)) {
            if let Ok(source) = Decoder::new(BufReader::new(file)) {
                let sink = Sink::connect_new(stream.mixer());
                sink.set_volume(request.volume);
                sink.append(source);
                sink.detach();
            } else {
                error!("Could not decode audio file: {}", audio_path);
            }
        } else {
            error!("Could not open audio file: {}", audio_path);
        }
    }
}

// Dedicated TTS player task that watches the queue and plays TTS sequentially
async fn tts_player_task(
    queue: TTSQueue,
    backend_tx: tokio::sync::mpsc::Sender<BackendToFrontendMessage>,
) {
    info!("TTS player task started");

    loop {
        // Wait for an item in the queue
        if let Some(item) = queue.pop().await {
            // Check if user is ignored
            if queue.is_user_ignored(&item.request.username).await {
                info!("Skipping TTS for ignored user: {}", item.request.username);
                continue;
            }

            // Set as currently playing
            queue.set_currently_playing(Some(item.clone())).await;

            // Send updated queue to frontend
            let queue_items = queue.get_all_with_current().await;
            let ui_queue: Vec<ui::TTSQueueItemUI> = queue_items
                .into_iter()
                .map(|item| ui::TTSQueueItemUI {
                    id: item.request.id,
                    username: item.request.username,
                    text: item.request.text,
                    language: item.request.language,
                })
                .collect();
            let _ = backend_tx
                .send(BackendToFrontendMessage::TTSQueueUpdated(ui_queue))
                .await;

            // Load current volume from config
            let volume = {
                let config = backend::config::load_config();
                config.tts.volume as f32
            };

            info!(
                "Playing TTS for user {} in language {}: {} chunk(s)",
                item.request.username,
                item.request.language,
                item.audio_chunks.len()
            );

            // Play audio chunks from memory
            let audio_chunks = item.audio_chunks.clone();
            let chunk_count = audio_chunks.len();
            let skip_flag = queue.get_skip_flag();

            match tokio::task::spawn_blocking(move || {
                // Create audio stream for TTS playback
                let stream = match rodio::OutputStreamBuilder::open_default_stream() {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to open TTS audio stream: {}", e);
                        return Err(format!("Failed to open audio stream: {}", e));
                    }
                };

                // Play each audio chunk synchronously
                for (index, chunk) in audio_chunks.iter().enumerate() {
                    // Check skip flag before playing each chunk
                    if skip_flag.load(std::sync::atomic::Ordering::SeqCst) {
                        info!("Skip detected, stopping playback");
                        return Ok(());
                    }

                    let cursor = std::io::Cursor::new(chunk.audio_data.clone());
                    if let Ok(source) = Decoder::new(BufReader::new(cursor)) {
                        let sink = Sink::connect_new(stream.mixer());
                        sink.set_volume(volume);
                        sink.append(source);

                        // Poll while waiting for playback to finish, checking skip flag
                        while !sink.empty() {
                            if skip_flag.load(std::sync::atomic::Ordering::SeqCst) {
                                info!("Skip detected during playback, stopping");
                                sink.stop();
                                return Ok(());
                            }
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }

                        info!("Finished playing TTS chunk {}/{}", index + 1, chunk_count);
                    } else {
                        error!(
                            "Could not decode TTS audio chunk {}/{}",
                            index + 1,
                            chunk_count
                        );
                    }

                    // Small delay between chunks
                    if chunk_count > 1 && index < chunk_count - 1 {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }

                Ok(())
            })
            .await
            {
                Ok(Ok(())) => {
                    info!("Finished TTS for user {}", item.request.username);
                }
                Ok(Err(e)) => {
                    error!("TTS playback error: {}", e);
                }
                Err(e) => {
                    error!("TTS task join error: {}", e);
                }
            }

            // Clear skip flag
            queue.clear_skip();

            // Clear currently playing
            queue.set_currently_playing(None).await;

            // Send updated queue to frontend
            let queue_items = queue.get_all_with_current().await;
            let ui_queue: Vec<ui::TTSQueueItemUI> = queue_items
                .into_iter()
                .map(|item| ui::TTSQueueItemUI {
                    id: item.request.id,
                    username: item.request.username,
                    text: item.request.text,
                    language: item.request.language,
                })
                .collect();
            let _ = backend_tx
                .send(BackendToFrontendMessage::TTSQueueUpdated(ui_queue))
                .await;
        } else {
            // Queue is empty, wait a bit before checking again
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}
