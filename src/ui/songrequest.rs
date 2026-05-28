use egui::{Color32, RichText, ScrollArea};

use crate::ui::{Chatbot, FrontendToBackendMessage};

impl Chatbot {
    pub fn show_song_request(&mut self, ui: &mut egui::Ui) {
        ui.heading("Song Request Queue");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let queue_len = self.song_queue.len();
            ui.label(format!("{} song(s) in queue", queue_len));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !self.song_queue.is_empty(),
                        egui::Button::new(RichText::new("Skip Current").color(Color32::from_rgb(255, 100, 100))),
                    )
                    .clicked()
                {
                    let _ = self.frontend_tx.try_send(FrontendToBackendMessage::SkipCurrentSong);
                }
            });
        });

        ui.separator();

        if self.song_queue.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No songs in queue").color(Color32::GRAY));
                ui.add_space(4.0);
                ui.label(RichText::new("Viewers can request songs with !sr <youtube_url>").color(Color32::GRAY).small());
            });
            return;
        }

        ScrollArea::vertical().show(ui, |ui| {
            let mut remove_id: Option<String> = None;

            for (index, song) in self.song_queue.iter().enumerate() {
                ui.add_space(4.0);
                egui::Frame::new()
                    .fill(Color32::from_rgb(35, 35, 50))
                    .inner_margin(egui::Margin::same(8))
                    .corner_radius(4)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Position number
                            ui.label(
                                RichText::new(format!("#{}", index + 1))
                                    .color(Color32::GRAY)
                                    .monospace(),
                            );

                            ui.vertical(|ui| {
                                // Title (clickable link to URL)
                                ui.hyperlink_to(
                                    RichText::new(&song.title).strong(),
                                    &song.url,
                                );
                                ui.horizontal(|ui| {
                                    if !song.channel.is_empty() {
                                        ui.label(
                                            RichText::new(&song.channel)
                                                .color(Color32::from_rgb(180, 180, 180))
                                                .small(),
                                        );
                                        ui.label(RichText::new("·").color(Color32::GRAY).small());
                                    }
                                    if !song.duration.is_empty() {
                                        ui.label(
                                            RichText::new(&song.duration)
                                                .color(Color32::from_rgb(180, 180, 180))
                                                .small(),
                                        );
                                        ui.label(RichText::new("·").color(Color32::GRAY).small());
                                    }
                                    ui.label(
                                        RichText::new(format!("by {}", song.requested_by))
                                            .color(Color32::from_rgb(130, 200, 255))
                                            .small(),
                                    );
                                });
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .button(RichText::new("✕").color(Color32::from_rgb(255, 80, 80)))
                                    .on_hover_text("Remove from queue")
                                    .clicked()
                                {
                                    remove_id = Some(song.id.clone());
                                }
                            });
                        });
                    });
            }

            if let Some(id) = remove_id {
                let _ = self.frontend_tx.try_send(FrontendToBackendMessage::RemoveSongRequest(id));
            }
        });
    }
}
