use egui::{CentralPanel, Color32, TopBottomPanel};

pub mod sfx;

enum Section {
    Home,
    Sfx,
    Tts,
    Settings,
}
#[derive(Debug)]
enum Roles {
    DEFAULT,
    VIP,
    SUBSCRIBER,
    MODERATOR,
}

pub enum BackendMessageAction {
    RemoveTTSLang(String),
    AddTTSLang(String),
    UpdateConfig {
        channel_name: String,
        auth_token: String,
    },
    UpdateSfxConfig(Config),
    UpdateTTSConfig(Config),
    ConnectToChat(String),
    DisconnectFromChat(String),
}

#[derive(Debug)]
pub enum FrontendMessageAction {
    GetTTSLangs,
    GetTTSConfig(Config),
    GetSfxConfig(Config),
    GetSfxList,
}
#[derive(Debug)]
struct Config {
    volume: u8,
    enabled: bool,
    permited_roles: Vec<Roles>,
}

struct ChatbotUILabels {
    bot_status: String,
    connect_button: String,
    tts_status: ButtonStatus,
    sfx_status: ButtonStatus,
}
#[derive(PartialEq)]
enum ButtonStatus {
    ON,
    OFF,
}

impl ButtonStatus {
    fn to_string(&self) -> String {
        match self {
            ButtonStatus::ON => "ON".to_string(),
            ButtonStatus::OFF => "OFF".to_string(),
        }
    }
}

enum LogLevel {
    INFO,
    WARN,
    ERROR,
}

impl LogLevel {
    fn color(&self) -> Color32 {
        match self {
            LogLevel::INFO => Color32::from_rgb(0, 255, 0),
            LogLevel::WARN => Color32::from_rgb(255, 255, 0),
            LogLevel::ERROR => Color32::from_rgb(255, 50, 0),
        }
    }
}
struct LogMessage {
    message: String,
    timestamp: String,
    log_level: LogLevel,
}

struct ChatbotConfig {
    channel_name: String,
    auth_token: String,
}

pub struct Chatbot {
    config: ChatbotConfig,
    selected_section: Section,
    frontend_tx: tokio::sync::mpsc::Sender<BackendMessageAction>,
    frontend_rx: tokio::sync::mpsc::Receiver<FrontendMessageAction>,
    labels: ChatbotUILabels,
    log_messages: Vec<LogMessage>,
}

impl Chatbot {
    pub fn new(
        channel_name: String,
        auth_token: String,
        frontend_tx: tokio::sync::mpsc::Sender<BackendMessageAction>,
        frontend_rx: tokio::sync::mpsc::Receiver<FrontendMessageAction>,
    ) -> Self {
        Self {
            config: ChatbotConfig {
                channel_name: channel_name.clone(),
                auth_token: auth_token.clone(),
            },
            selected_section: Section::Home,
            frontend_tx: frontend_tx,
            frontend_rx: frontend_rx,
            labels: ChatbotUILabels {
                bot_status: "Disconnected".to_string(),
                connect_button: "Connect".to_string(),
                tts_status: ButtonStatus::ON,
                sfx_status: ButtonStatus::ON,
            },
            log_messages: Vec::new(),
        }
    }

    fn show_sfx(&mut self, ui: &mut egui::Ui) {
        ui.set_height(ui.available_height());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui: &mut egui::Ui| {
                    ui.label("SFX status: ");
                    if ui.button(self.labels.sfx_status.to_string()).clicked() {
                        if self.labels.sfx_status == ButtonStatus::ON {
                            self.labels.sfx_status = ButtonStatus::OFF;
                        } else {
                            self.labels.sfx_status = ButtonStatus::ON;
                        }
                    }
                });
                ui.add_space(10.0);
                ui.label("SFX volume (0-1 range):");
                ui.add(egui::Slider::new(&mut 0.92, 0.0..=1.0));
                ui.add_space(10.0);
                ui.label("SFX permissions:");
                ui.checkbox(&mut false, "Subs");
                ui.checkbox(&mut false, "VIPS");
                ui.checkbox(&mut false, "Mods");
                ui.add_space(350.0);
            });
            ui.add_space(250.0);
            ui.separator();
            ui.vertical(|ui| {
                ui.set_height(ui.available_height());
                ui.heading(
                    egui::widget_text::RichText::new("Available sounds").color(Color32::WHITE),
                );
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() - 100.0)
                    .max_width(ui.available_width())
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        for i in 0..100 {
                            ui.horizontal(|ui| {
                                ui.label(i.to_string());
                                ui.label("sound name");
                            });
                            ui.separator();
                        }
                    });
            });
        });
    }

    fn show_tts(&mut self, ui: &mut egui::Ui) {
        ui.set_height(ui.available_height());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui: &mut egui::Ui| {
                    ui.label("TTS status: ");
                    if ui.button(self.labels.tts_status.to_string()).clicked() {
                        if self.labels.tts_status == ButtonStatus::ON {
                            self.labels.tts_status = ButtonStatus::OFF;
                        } else {
                            self.labels.tts_status = ButtonStatus::ON;
                        }
                    }
                });
                ui.add_space(10.0);
                ui.label("TTS volume (0-1 range):");
                ui.add(egui::Slider::new(&mut 0.92, 0.0..=1.0));
                ui.add_space(10.0);
                ui.label("TTS permissions:");
                ui.checkbox(&mut false, "Subs");
                ui.checkbox(&mut false, "VIPS");
                ui.checkbox(&mut false, "Mods");
                ui.add_space(350.0);
            });
            ui.add_space(250.0);
            ui.separator();
            ui.vertical(|ui| {
                ui.set_height(ui.available_height());
                let available_height = ui.available_height();
                let table = egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .resizable(false)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(egui_extras::Column::auto())
                    .column(egui_extras::Column::initial(200.0))
                    .column(egui_extras::Column::auto())
                    .min_scrolled_height(0.0)
                    .max_scroll_height(available_height);

                table
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("No.");
                        });
                        header.col(|ui| {
                            ui.strong("Language name");
                        });
                        header.col(|ui| {
                            ui.strong("Enabled");
                        });
                    })
                    .body(|mut body| {
                        for row_index in 1..100 {
                            let row_height = 18.0;
                            body.row(row_height, |mut row| {
                                row.col(|ui| {
                                    ui.label(row_index.to_string());
                                });
                                row.col(|ui| {
                                    ui.label("test");
                                });
                                row.col(|ui| {
                                    ui.checkbox(&mut false, "");
                                });
                            });
                        }
                    })
            });
        });
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Channel name:");
                ui.text_edit_singleline(&mut self.config.channel_name);
            });
            ui.horizontal(|ui| {
                ui.label("Auth token:");
                ui.text_edit_singleline(&mut self.config.auth_token);
            });
            if ui.button("Save").clicked() {
                let _ = self.frontend_tx.send(BackendMessageAction::UpdateConfig {
                    channel_name: self.config.channel_name.clone(),
                    auth_token: self.config.auth_token.clone(),
                });
            }
        });
    }
}

impl eframe::App for Chatbot {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.set_height(25.0);
                ui.spacing_mut().item_spacing.x = 5.0;
                ui.horizontal(|ui| {
                    ui.image(egui::include_image!("../../assets/img/logo.png"));
                    ui.label("Yambot");
                });
                ui.add_space(ui.available_width() - (ui.available_width() - 495.0));
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
            ui.vertical_centered(|ui| {
                ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                ui.hyperlink_to("Source code", "https://www.github.com/xyamii/yambot");
            });
        });

        while let Ok(message) = self.frontend_rx.try_recv() {
            match message {
                FrontendMessageAction::GetSfxConfig(config) => {
                    println!("Getting sfx config {:?}", config);
                }
                FrontendMessageAction::GetTTSConfig(config) => {
                    println!("Getting tts config {:?}", config);
                }
                _ => {
                    println!("Received message");
                }
            }
        }

        ctx.request_repaint();
    }
}
