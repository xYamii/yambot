use egui::{Color32, RichText, ScrollArea};

use crate::ui::{Chatbot, FrontendToBackendMessage};

impl Chatbot {
    pub fn show_song_request(&mut self, ui: &mut egui::Ui) {
        ui.heading("Song Request");
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            // Left: Settings
            ui.vertical(|ui| {
                ui.set_width(230.0);

                ui.label(RichText::new("Settings").strong());
                ui.add_space(4.0);

                let old_enabled = self.song_request_config.enabled;
                ui.checkbox(&mut self.song_request_config.enabled, "Enable song requests");
                if self.song_request_config.enabled != old_enabled {
                    let _ = self.frontend_tx.try_send(
                        FrontendToBackendMessage::UpdateSongRequestConfig(
                            self.song_request_config.clone(),
                        ),
                    );
                }

                ui.add_space(8.0);
                ui.label("YouTube API key:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.song_request_config.youtube_api_key)
                        .password(true)
                        .hint_text("AIza..."),
                );
                ui.label(
                    RichText::new("Optional — used to fetch song titles")
                        .color(Color32::GRAY)
                        .small(),
                );
                if ui.button("Save API key").clicked() {
                    let _ = self.frontend_tx.try_send(
                        FrontendToBackendMessage::UpdateSongRequestConfig(
                            self.song_request_config.clone(),
                        ),
                    );
                }

                ui.add_space(8.0);
                ui.label(RichText::new("Overlay").strong());

                let old_title = self.song_request_config.title_visible;
                ui.checkbox(&mut self.song_request_config.title_visible, "Show song title");
                if self.song_request_config.title_visible != old_title {
                    let _ = self.frontend_tx.try_send(
                        FrontendToBackendMessage::UpdateSongRequestConfig(
                            self.song_request_config.clone(),
                        ),
                    );
                }

                let old_video = self.song_request_config.video_visible;
                ui.checkbox(&mut self.song_request_config.video_visible, "Show video");
                if self.song_request_config.video_visible != old_video {
                    let _ = self.frontend_tx.try_send(
                        FrontendToBackendMessage::UpdateSongRequestConfig(
                            self.song_request_config.clone(),
                        ),
                    );
                }

                ui.add_space(8.0);
                ui.label(RichText::new("Volume").strong());
                ui.label(
                    RichText::new("Use !volume X in chat (mods only)")
                        .color(Color32::GRAY)
                        .small(),
                );
            });

            ui.separator();

            // Right: Queue
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let queue_len = self.song_queue.len();
                    ui.label(format!("{} song(s) in queue", queue_len));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                !self.song_queue.is_empty(),
                                egui::Button::new(
                                    RichText::new("Skip").color(Color32::from_rgb(255, 100, 100)),
                                ),
                            )
                            .clicked()
                        {
                            self.song_paused = false;
                            let _ = self
                                .frontend_tx
                                .try_send(FrontendToBackendMessage::SkipCurrentSong);
                        }

                        let (pause_label, pause_color) = if self.song_paused {
                            ("▶ Resume", Color32::from_rgb(100, 200, 100))
                        } else {
                            ("⏸ Pause", Color32::from_rgb(255, 200, 50))
                        };
                        if ui
                            .add_enabled(
                                !self.song_queue.is_empty(),
                                egui::Button::new(
                                    RichText::new(pause_label).color(pause_color),
                                ),
                            )
                            .clicked()
                        {
                            if self.song_paused {
                                self.song_paused = false;
                                let _ = self
                                    .frontend_tx
                                    .try_send(FrontendToBackendMessage::ResumeSong);
                            } else {
                                self.song_paused = true;
                                let _ = self
                                    .frontend_tx
                                    .try_send(FrontendToBackendMessage::PauseSong);
                            }
                        }
                    });
                });

                ui.separator();

                if self.song_queue.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("No songs in queue").color(Color32::GRAY));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Viewers can request songs with !sr <youtube_url>")
                                .color(Color32::GRAY)
                                .small(),
                        );
                    });
                } else {
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
                                        ui.label(
                                            RichText::new(format!("#{}", index + 1))
                                                .color(Color32::GRAY)
                                                .monospace(),
                                        );
                                        ui.vertical(|ui| {
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
                                                    ui.label(
                                                        RichText::new("·")
                                                            .color(Color32::GRAY)
                                                            .small(),
                                                    );
                                                }
                                                if !song.duration.is_empty() {
                                                    ui.label(
                                                        RichText::new(&song.duration)
                                                            .color(Color32::from_rgb(180, 180, 180))
                                                            .small(),
                                                    );
                                                    ui.label(
                                                        RichText::new("·")
                                                            .color(Color32::GRAY)
                                                            .small(),
                                                    );
                                                }
                                                ui.label(
                                                    RichText::new(format!(
                                                        "by {}",
                                                        song.requested_by
                                                    ))
                                                    .color(Color32::from_rgb(130, 200, 255))
                                                    .small(),
                                                );
                                            });
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(
                                                        RichText::new("✕")
                                                            .color(Color32::from_rgb(255, 80, 80)),
                                                    )
                                                    .on_hover_text("Remove from queue")
                                                    .clicked()
                                                {
                                                    remove_id = Some(song.id.clone());
                                                }
                                            },
                                        );
                                    });
                                });
                        }

                        if let Some(id) = remove_id {
                            let _ = self
                                .frontend_tx
                                .try_send(FrontendToBackendMessage::RemoveSongRequest(id));
                        }
                    });
                }
            });
        });
    }
}
