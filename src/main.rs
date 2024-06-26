use eframe::egui::{self, CentralPanel, TopBottomPanel};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::PrivmsgMessage;
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};

const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;

enum Section {
    Home,
    Sfx,
    Tts,
    Settings,
}

enum MessageAction {
    RefreshSfxList,
    UpdateConfig,
    UpdateSfx,
    UpdateTTSConfig,
    UpdateTTSLangList,
    ConnectToChat,
    DisconnectFromChat,
}

struct TransportMessage {
    action: MessageAction,
    payload: Option<String>,
}

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
    connect_button_label: String,
    selected_section: Section,
    frontend_tx: tokio::sync::mpsc::Sender<TransportMessage>,
    frontend_rx: tokio::sync::mpsc::Receiver<TransportMessage>,
}

impl Chatbot {
    fn new(
        channel_name: String,
        frontend_tx: tokio::sync::mpsc::Sender<TransportMessage>,
        frontend_rx: tokio::sync::mpsc::Receiver<TransportMessage>,
    ) -> Self {
        Self {
            channel_name: channel_name,
            connect_button_label: "Connect".to_string(),
            selected_section: Section::Home,
            frontend_tx: frontend_tx,
            frontend_rx: frontend_rx,
        }
    }

    fn show_home(&mut self, ui: &mut egui::Ui) {
        ui.heading("Home Section");
        if ui.button(&self.connect_button_label).clicked() {
            if self.connect_button_label == "Connect" {
                self.connect_button_label = "Disconnect".to_string();
                let _ = self.frontend_tx.send(TransportMessage {
                    action: MessageAction::ConnectToChat,
                    payload: None,
                });
            } else {
                self.connect_button_label = "Connect".to_string();
                let _ = self.frontend_tx.send(TransportMessage {
                    action: MessageAction::DisconnectFromChat,
                    payload: None,
                });
            };
        }
        ui.label("Content for the HOME section goes here.");
    }

    fn show_sfx(&self, ui: &mut egui::Ui) {
        ui.heading("SFX Section");
        ui.label(&self.channel_name);
    }

    fn show_tts(&self, ui: &mut egui::Ui) {
        ui.heading("TTS Section");
        ui.label("Content for the TTS section goes here.");
    }

    fn show_settings(&self, ui: &mut egui::Ui) {
        ui.heading("Settings Section");
        ui.label("Content for the SETTINGS section goes here.");
    }
}

impl eframe::App for Chatbot {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.set_height(25.0);
                ui.spacing_mut().item_spacing.x = 5.0;
                ui.horizontal(|ui| {
                    ui.image(egui::include_image!("../assets/img/logo.png"));
                    ui.label("Yambot");
                });

                ui.add_space(ui.available_width() - (ui.available_width() - 495.0)); // Adjust the space value as needed
                ui.horizontal(|ui| {
                    if ui.button("HOME").clicked() {
                        self.selected_section = Section::Home;
                    }
                    if ui.button("SFX").clicked() {
                        self.selected_section = Section::Sfx;
                    }
                    if ui.button("TTS").clicked() {
                        self.selected_section = Section::Tts;
                    }
                    if ui.button("SETTINGS").clicked() {
                        self.selected_section = Section::Settings;
                    }
                });
            });
        });

        CentralPanel::default().show(ctx, |ui| match self.selected_section {
            Section::Home => self.show_home(ui),
            Section::Sfx => self.show_sfx(ui),
            Section::Tts => self.show_tts(ui),
            Section::Settings => self.show_settings(ui),
        });

        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            // this fucking shit doesnt want to center REEEE
            ui.vertical_centered(|ui| {
                ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                ui.hyperlink_to("Source code", "https://www.github.com/xyamii/yambot");
            });
        });

        // loop receiving  messages

        ctx.request_repaint();
    }
}

#[tokio::main]
async fn main() {
    let (backend_tx, frontend_rx) = tokio::sync::mpsc::channel(100);
    let (frontend_tx, backend_rx) = tokio::sync::mpsc::channel(100);
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_resizable(false),
        ..Default::default()
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
            Box::new(Chatbot::new("yamii".to_string(), frontend_tx, frontend_rx))
        }),
    );
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
