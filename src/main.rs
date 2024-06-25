use eframe::egui::{self, CentralPanel, ScrollArea, TopBottomPanel};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::PrivmsgMessage;
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};

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

struct Chatbot {
    channel_name: String,
    messages: Arc<Mutex<Vec<ChatMessage>>>,
}

impl Default for Chatbot {
    fn default() -> Self {
        Self {
            channel_name: String::new(),
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl eframe::App for Chatbot {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Channel:");
                ui.text_edit_singleline(&mut self.channel_name);
                if ui.button("Connect").clicked() {
                    if !self.channel_name.is_empty() {
                        let channel = self.channel_name.clone();
                        let messages = Arc::clone(&self.messages);
                        tokio::spawn(async move {
                            handle_messages(channel, messages).await;
                        });
                    }
                }
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                let messages: std::sync::MutexGuard<Vec<ChatMessage>> = self.messages.lock().unwrap();
                for msg in messages.iter() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: ", &msg.username));
                        ui.label(&msg.message_text);
                    });
                }
            });
        });

        ctx.request_repaint();
    }
}

#[tokio::main]
async fn main() {
    let app = Chatbot::default();
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native("Yambot", native_options, Box::new(|_| Box::new(app)));
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
