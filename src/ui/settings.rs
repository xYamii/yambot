use super::{BackendMessageAction, Chatbot};

impl Chatbot {
    pub fn show_settings(&mut self, ui: &mut egui::Ui) {
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
                let _ = self.frontend_tx.try_send(BackendMessageAction::UpdateConfig {
                    channel_name: self.config.channel_name.clone(),
                    auth_token: self.config.auth_token.clone(),
                }).unwrap();
            }
        });
    }
}
