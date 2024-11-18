use backend::config::AppConfig;
use eframe::egui::{self};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::PrivmsgMessage;
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};
use ui::BackendMessageAction;

pub mod backend;
pub mod ui;
use log::{error, info};

const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub message_id: String,
    pub message_text: String,
    pub badges: Vec<String>,
    pub username: String,
}

impl From<PrivmsgMessage> for ChatMessage {
    fn from(privmsg: PrivmsgMessage) -> Self {
        let badges = privmsg
            .badges
            .into_iter()
            .map(|badge| format!("{}-{}", badge.name, badge.version))
            .collect();
        ChatMessage {
            message_id: privmsg.message_id,
            message_text: privmsg.message_text,
            badges,
            username: privmsg.sender.login,
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
    tokio::spawn(async move {
        handle_backend_messages(backend_rx).await;
    });
    info!("Starting chatbot");
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
            )))
        }),
    )
    .map_err(|e| error!("Error: {:?}", e));
}

async fn handle_messages(channel_name: String, messages: Arc<Mutex<Vec<ChatMessage>>>) {
    let config: ClientConfig<StaticLoginCredentials> = ClientConfig::default();
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);
    client.join(channel_name.clone()).unwrap();

    while let Some(message) = incoming_messages.recv().await {
        match message {
            twitch_irc::message::ServerMessage::Privmsg(privmsg) => {
                let chat_message: ChatMessage = privmsg.into();
                println!("Message: {:?}", chat_message);
                messages.lock().unwrap().push(chat_message);
            }
            twitch_irc::message::ServerMessage::Join(join_msg) => {
                println!("User joined: {}", join_msg.user_login);
            }
            twitch_irc::message::ServerMessage::Part(part_msg) => {
                println!("User left: {}", part_msg.user_login);
            }
            twitch_irc::message::ServerMessage::Whisper(whisper_message) => {
                println!(
                    "User {}, whispered message {}",
                    whisper_message.sender.login, whisper_message.message_text
                );
            }
            _ => {
                println!("Received other message: {:?}", message);
            }
        }
    }
}
async fn handle_backend_messages(
    mut backend_rx: tokio::sync::mpsc::Receiver<BackendMessageAction>,
) {
    while let Some(message) = backend_rx.recv().await {
        match message {
            BackendMessageAction::UpdateTTSConfig(config) => {
                let current_config: AppConfig = backend::config::load_config();
                backend::config::save_config(&AppConfig {
                    chatbot: current_config.chatbot,
                    sfx: current_config.sfx,
                    tts: config,
                });
            }
            BackendMessageAction::UpdateSfxConfig(config) => {
                let current_config: AppConfig = backend::config::load_config();
                backend::config::save_config(&AppConfig {
                    chatbot: current_config.chatbot,
                    sfx: config,
                    tts: current_config.tts,
                });
            }
            BackendMessageAction::UpdateConfig(config) => {
                let current_config: AppConfig = backend::config::load_config();
                backend::config::save_config(&AppConfig {
                    chatbot: config,
                    sfx: current_config.sfx,
                    tts: current_config.tts,
                });
            }
            _ => {
                println!("Received other message: {:?}", message);
            }
        }
    }
}
