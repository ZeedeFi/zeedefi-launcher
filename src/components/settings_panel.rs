//! `src/components/settings_panel.rs` — tiroir de paramètres (côté droit).
//!
//! Navigation clavier/manette (haut/bas, A, B) et souris. Le panneau ne modifie
//! rien lui-même : il retourne une `SettingAction` que `app.rs` applique.

use egui::{Color32, Id, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

use crate::app::ConnectionStatus;
use crate::components::icons;
use crate::input_handler::{Action, Direction};
use crate::theme;

const ROW_COUNT: usize = 7;
const ROW_HEIGHT: f32 = 60.0;
const SECTION_HEIGHT: f32 = 34.0;
const CONTENT_TOP: f32 = 106.0;

/// État du panneau de paramètres.
#[derive(Debug, Clone)]
pub struct SettingsPanel {
    pub open: bool,
    anim: f32,
    pub selected: usize,
    pub blur_enabled: bool,
    pub show_fps: bool,
    pub connection: ConnectionStatus,
    pub game_count: usize,
    pub error_code: Option<u32>,
    confirm: Option<usize>,
    ignore_outside_click: bool,
}

/// Action produite par le panneau.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingAction {
    ToggleBlur,
    ToggleFps,
    RefreshLibrary,
    Reconnect,
    Sleep,
    Shutdown,
    Quit,
    Close,
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self {
            open: false,
            anim: 0.0,
            selected: 0,
            blur_enabled: true,
            show_fps: false,
            connection: ConnectionStatus::Disconnected,
            game_count: 0,
            error_code: None,
            confirm: None,
            ignore_outside_click: false,
        }
    }

    /// Ouvre le panneau en évitant qu'un clic d'ouverture ne le referme aussitôt.
    pub fn open_now(&mut self) {
        self.open = true;
        self.ignore_outside_click = true;
        self.confirm = None;
    }

    /// Indique si le panneau est visible ou en cours d'animation.
    pub fn is_animating(&self) -> bool {
        self.open || self.anim > 0.0
    }

    pub fn close_now(&mut self) {
        self.open = false;
        self.confirm = None;
    }

    /// Gère une action d'entrée. Retourne une action sémantique si applicable.
    pub fn handle_action(&mut self, action: &Action) -> Option<SettingAction> {
        if !self.open {
            return None;
        }
        match action {
            Action::Navigate(Direction::Up) => {
                self.confirm = None;
                self.selected = self.selected.saturating_sub(1);
                None
            }
            Action::Navigate(Direction::Down) => {
                self.confirm = None;
                if self.selected + 1 < ROW_COUNT {
                    self.selected += 1;
                }
                None
            }
            Action::Confirm => self.activate(self.selected),
            Action::Back => Some(SettingAction::Close),
            _ => None,
        }
    }

    fn activate(&mut self, index: usize) -> Option<SettingAction> {
        match index {
            0 => Some(SettingAction::ToggleBlur),
            1 => Some(SettingAction::ToggleFps),
            2 => Some(SettingAction::RefreshLibrary),
            3 => Some(SettingAction::Reconnect),
            4 => Some(SettingAction::Sleep),
            5 | 6 => {
                let risky = Some(index);
                if self.confirm == risky {
                    self.confirm = None;
                    Some(if index == 5 {
                        SettingAction::Shutdown
                    } else {
                        SettingAction::Quit
                    })
                } else {
                    self.confirm = risky;
                    None
                }
            }
            _ => None,
        }
    }

    /// Dessine le panneau et retourne l'action déclenchée, le cas échéant.
    pub fn draw(
        &mut self,
        context: &egui::Context,
        viewport: Rect,
        delta_seconds: f32,
    ) -> Option<SettingAction> {
        let target = if self.open { 1.0 } else { 0.0 };
        let speed = (delta_seconds * 14.0).clamp(0.0, 1.0);
        self.anim += (target - self.anim) * speed.max(0.12);
        if !self.open && self.anim < 0.005 {
            self.anim = 0.0;
            return None;
        }

        let palette = theme::Palette::default();
        let width = theme::SETTINGS_PANEL_WIDTH
            .min(viewport.width() * 0.94)
            .max(320.0);
        let panel_x = viewport.max.x - width * self.anim;

        // Voile d'arrière-plan.
        let backdrop = context.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            Id::new("settings_backdrop"),
        ));
        backdrop.rect_filled(
            viewport,
            Rounding::ZERO,
            theme::with_alpha(Color32::BLACK, self.anim * 0.52),
        );

        let mut action = None;
        let area = egui::Area::new(Id::new("settings_panel_area"))
            .fixed_pos(Pos2::new(panel_x, viewport.min.y))
            .order(egui::Order::Foreground);
        area.show(context, |ui| {
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(width, viewport.height()),
                Sense::hover(),
            );
            let painter = ui.painter_at(rect);
            let rounding = Rounding {
                nw: 20.0,
                sw: 20.0,
                ne: 0.0,
                se: 0.0,
            };
            painter.rect_filled(
                rect,
                rounding,
                theme::lerp_color(palette.surface, palette.surface_elevated, 0.55),
            );
            painter.line_segment(
                [rect.min, Pos2::new(rect.min.x, rect.max.y)],
                Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.10)),
            );

            // --- En-tête ---
            painter.text(
                Pos2::new(rect.min.x + 28.0, rect.min.y + 34.0),
                egui::Align2::LEFT_BOTTOM,
                "Paramètres",
                theme::bold(27.0),
                palette.text_primary,
            );
            painter.text(
                Pos2::new(rect.min.x + 28.0, rect.min.y + 58.0),
                egui::Align2::LEFT_TOP,
                "Personnalisez votre expérience ZeedeFi",
                theme::font(13.0),
                palette.text_secondary,
            );

            let close_rect = Rect::from_center_size(
                Pos2::new(rect.max.x - 40.0, rect.min.y + 46.0),
                Vec2::splat(34.0),
            );
            let close_response = ui
                .interact(close_rect, Id::new("settings_close"), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if close_response.hovered() {
                painter.circle_filled(
                    close_rect.center(),
                    17.0,
                    theme::with_alpha(Color32::WHITE, 0.10),
                );
            }
            icons::close(
                &painter,
                close_rect.shrink(9.0),
                if close_response.hovered() {
                    palette.text_primary
                } else {
                    palette.text_secondary
                },
            );
            if close_response.clicked() {
                action = Some(SettingAction::Close);
            }

            painter.line_segment(
                [
                    Pos2::new(rect.min.x + 20.0, rect.min.y + 86.0),
                    Pos2::new(rect.max.x - 20.0, rect.min.y + 86.0),
                ],
                Stroke::new(1.0_f32, palette.divider),
            );

            // --- Contenu ---
            let left = rect.min.x + 24.0;
            let row_width = width - 48.0;
            let mut y = rect.min.y + CONTENT_TOP;
            let mut index = 0usize;

            let sections: [(&str, usize); 3] = [
                ("AFFICHAGE", 2),
                ("BIBLIOTHÈQUE", 2),
                ("SYSTÈME", 3),
            ];
            for (section, count) in sections {
                painter.text(
                    Pos2::new(left + 4.0, y),
                    egui::Align2::LEFT_TOP,
                    section,
                    theme::bold(10.5),
                    theme::with_alpha(palette.text_muted, 0.95),
                );
                y += SECTION_HEIGHT;

                for _ in 0..count {
                    let row_rect = Rect::from_min_size(
                        Pos2::new(left, y),
                        Vec2::new(row_width, ROW_HEIGHT),
                    );
                    let response = ui
                        .interact(
                            row_rect,
                            Id::new(("settings_row", index)),
                            Sense::click(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let hovered = response.hovered();
                    let selected = index == self.selected;
                    let active = hovered || selected;
                    if active {
                        let row_rounding = Rounding::same(12.0);
                        painter.rect_filled(
                            row_rect,
                            row_rounding,
                            theme::with_alpha(
                                Color32::WHITE,
                                if hovered { 0.075 } else { 0.05 },
                            ),
                        );
                        if selected {
                            painter.rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(row_rect.min.x + 6.0, row_rect.min.y + 12.0),
                                    Vec2::new(3.0, ROW_HEIGHT - 24.0),
                                ),
                                Rounding::same(2.0),
                                theme::ACCENT,
                            );
                        }
                    }

                    let (title, subtitle) = row_texts(index);
                    painter.text(
                        Pos2::new(row_rect.min.x + 18.0, row_rect.center().y - 9.0),
                        egui::Align2::LEFT_CENTER,
                        title,
                        theme::font(14.5),
                        if index >= 5 {
                            palette.danger
                        } else {
                            palette.text_primary
                        },
                    );
                    painter.text(
                        Pos2::new(row_rect.min.x + 18.0, row_rect.center().y + 11.0),
                        egui::Align2::LEFT_CENTER,
                        subtitle,
                        theme::font(11.5),
                        palette.text_muted,
                    );

                    let trailing_center = Pos2::new(row_rect.max.x - 34.0, row_rect.center().y);
                    match index {
                        0 | 1 => {
                            let value = if index == 0 {
                                self.blur_enabled
                            } else {
                                self.show_fps
                            };
                            let t = ui.ctx().animate_bool(
                                Id::new(("settings_toggle", index)),
                                value,
                            );
                            draw_toggle(&painter, trailing_center, t);
                        }
                        2 => {
                            painter.text(
                                Pos2::new(trailing_center.x - 14.0, trailing_center.y),
                                egui::Align2::RIGHT_CENTER,
                                format!("{} jeux", self.game_count),
                                theme::font(11.5),
                                palette.text_secondary,
                            );
                            icons::arrow_right(
                                &painter,
                                Rect::from_center_size(trailing_center, Vec2::splat(16.0)),
                                palette.text_secondary,
                            );
                        }
                        3 => {
                            let (label, color) = match &self.connection {
                                ConnectionStatus::Connected => ("Connecté", palette.success),
                                ConnectionStatus::Connecting => ("Connexion…", palette.warning),
                                ConnectionStatus::Disconnected => ("Hors ligne", palette.text_muted),
                                ConnectionStatus::Failed { .. } => ("Erreur", palette.danger),
                            };
                            let text_width = painter
                                .layout_no_wrap(
                                    label.to_string(),
                                    theme::font(11.5),
                                    color,
                                )
                                .size()
                                .x;
                            painter.circle_filled(
                                Pos2::new(
                                    row_rect.max.x - 26.0 - text_width - 12.0,
                                    row_rect.center().y,
                                ),
                                3.2,
                                color,
                            );
                            painter.text(
                                Pos2::new(row_rect.max.x - 26.0, row_rect.center().y),
                                egui::Align2::RIGHT_CENTER,
                                label,
                                theme::font(11.5),
                                color,
                            );
                        }
                        _ => {
                            if self.confirm == Some(index) {
                                painter.text(
                                    Pos2::new(row_rect.max.x - 22.0, row_rect.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    "Confirmer",
                                    theme::bold(11.0),
                                    palette.warning,
                                );
                            } else {
                                icons::arrow_right(
                                    &painter,
                                    Rect::from_center_size(trailing_center, Vec2::splat(16.0)),
                                    palette.text_muted,
                                );
                            }
                        }
                    }

                    if response.clicked() {
                        self.selected = index;
                        if let Some(triggered) = self.activate(index) {
                            action = Some(triggered);
                        } else {
                            self.confirm = Some(index);
                        }
                    }

                    y += ROW_HEIGHT;
                    index += 1;
                }
                y += 8.0;
            }

            // --- Pied de panneau : état de connexion + version ---
            let footer_y = rect.max.y - 62.0;
            painter.line_segment(
                [
                    Pos2::new(rect.min.x + 20.0, footer_y - 14.0),
                    Pos2::new(rect.max.x - 20.0, footer_y - 14.0),
                ],
                Stroke::new(1.0_f32, palette.divider),
            );
            let (status, color) = match &self.connection {
                ConnectionStatus::Connected => ("Démon ZeedeFi connecté".to_string(), palette.success),
                ConnectionStatus::Connecting => ("Connexion au démon…".to_string(), palette.warning),
                ConnectionStatus::Disconnected => ("Démon hors ligne".to_string(), palette.text_muted),
                ConnectionStatus::Failed { code } => {
                    if *code == crate::zeedefi_client::ErrorCode::SocketMissing as u32
                        || *code == crate::zeedefi_client::ErrorCode::ConnectionRefused as u32
                    {
                        ("Démon hors ligne".to_string(), palette.danger)
                    } else {
                        (format!("Erreur #{code}"), palette.danger)
                    }
                }
            };
            painter.circle_filled(Pos2::new(rect.min.x + 30.0, footer_y), 3.5, color);
            painter.text(
                Pos2::new(rect.min.x + 42.0, footer_y),
                egui::Align2::LEFT_CENTER,
                status,
                theme::font(12.0),
                palette.text_secondary,
            );
            painter.text(
                Pos2::new(rect.max.x - 24.0, footer_y),
                egui::Align2::RIGHT_CENTER,
                format!("v{}", env!("CARGO_PKG_VERSION")),
                theme::font(11.0),
                palette.text_muted,
            );
        });

        // Clic en dehors du tiroir : fermeture (sauf au frame d'ouverture).
        if self.ignore_outside_click {
            self.ignore_outside_click = false;
        } else if action.is_none() {
            let clicked_outside = context.input(|input| {
                input.pointer.primary_clicked()
                    && input
                        .pointer
                        .interact_pos()
                        .map(|pos| !Rect::from_min_size(
                            Pos2::new(panel_x, viewport.min.y),
                            Vec2::new(width, viewport.height()),
                        ).contains(pos))
                        .unwrap_or(false)
            });
            if clicked_outside {
                action = Some(SettingAction::Close);
            }
        }

        action
    }
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}

fn row_texts(index: usize) -> (&'static str, &'static str) {
    match index {
        0 => ("Flou d'arrière-plan", "Adoucit l'artwork du jeu en arrière-plan"),
        1 => ("Afficher les FPS", "Compteur de performance en haut à gauche"),
        2 => ("Rafraîchir la bibliothèque", "Redemande la liste des jeux au démon"),
        3 => ("Reconnecter le démon", "Relance la connexion au socket Unix"),
        4 => ("Mettre en veille", "Délégué au démon ZeedeFi"),
        5 => ("Éteindre", "Arrêt complet de la machine (demande au démon)"),
        _ => ("Quitter le launcher", "Ferme uniquement cette interface"),
    }
}

fn draw_toggle(painter: &egui::Painter, center: Pos2, t: f32) {
    let size = Vec2::new(42.0, 24.0);
    let track = Rect::from_center_size(center, size);
    let rounding = Rounding::same(size.y * 0.5);
    let off = theme::with_alpha(Color32::WHITE, 0.16);
    painter.rect_filled(track, rounding, theme::lerp_color(off, theme::ACCENT, t));
    let knob_x = track.min.x + size.y * 0.5 + (size.x - size.y) * t;
    painter.circle_filled(
        Pos2::new(knob_x, track.center().y),
        size.y * 0.36,
        theme::lerp_color(Color32::from_rgb(0xD6, 0xDC, 0xE6), Color32::WHITE, t),
    );
}
