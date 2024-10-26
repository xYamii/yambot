use egui::Color32;

use super::{ FrontendToBackendMessage, Chatbot, LogLevel, LogMessage };

impl Chatbot {
    pub fn show_home(&mut self, ui: &mut egui::Ui) {
        ui.set_min_height(ui.max_rect().height());
        ui.set_min_width(ui.max_rect().width());
        ui.horizontal(|ui| {
            if ui.button(&self.labels.connect_button).clicked() {
                if self.labels.connect_button == "Connect" {
                    if self.config.auth_token == "" {
                        self.log_messages.push(LogMessage {
                            message: "Tried to connect to the chat without auth token".to_string(),
                            timestamp: chrono::Local::now().to_string(),
                            log_level: LogLevel::ERROR,
                        });
                        return;
                    }
                    self.labels.connect_button = "Disconnect".to_string();
                    let _ = self.frontend_tx
                        .try_send(
                            FrontendToBackendMessage::ConnectToChat(
                                self.config.channel_name.clone()
                            )
                        )
                        .unwrap();
                    self.labels.bot_status = "Connected".to_string();
                } else {
                    self.labels.connect_button = "Connect".to_string();
                    let _ = self.frontend_tx
                        .try_send(
                            FrontendToBackendMessage::DisconnectFromChat(
                                self.config.channel_name.clone()
                            )
                        )
                        .unwrap();
                    self.labels.bot_status = "Disconnected".to_string();
                }
            }
            ui.label(format!("Status: {}", self.labels.bot_status));
        });
        ui.separator();
        ui.heading(egui::widget_text::RichText::new("Bot logs").color(Color32::WHITE));
        egui::ScrollArea
            ::vertical()
            .max_height(ui.available_height() - 100.0)
            .max_width(ui.available_width())
            .auto_shrink(false)
            .show(ui, |ui| {
                for mesasge in self.log_messages.iter() {
                    ui.horizontal(|ui| {
                        ui.label(&mesasge.timestamp);
                        ui.label(
                            egui::widget_text::RichText
                                ::new(&mesasge.message)
                                .color(mesasge.log_level.color())
                        );
                    });
                    ui.separator();
                }
            });
        // for testing purposes
        if ui.button("test".to_string()).clicked() {
            let _ = self
                .frontend_tx
                .try_send(FrontendToBackendMessage::PlaySound("test.wav".to_string()))
                .unwrap();
        }
    }
}
