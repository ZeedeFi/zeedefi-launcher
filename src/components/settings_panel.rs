//! `src/components/settings_panel.rs` — écran des paramètres (plein écran).
//!
//! Canevas qui masque toute l'interface d'accueil. Une liste verticale de
//! rubriques (Internet, Mise à jour, Compte, Bibliothèque, Affichage, Système)
//! occupe le centre de l'écran, dans le même langage visuel que l'accueil.
//! Le panneau ne modifie rien lui-même : il retourne une `SettingAction` que
//! `app.rs` applique. Navigation exclusivement clavier/manette.

use egui::{Color32, Id, LayerId, Order, Pos2, Rect, Rounding, Stroke, Vec2};

use crate::app::ConnectionStatus;
use crate::input_handler::{Action, Direction};
use crate::theme;

const ROW_HEIGHT: f32 = 64.0;
const ROW_GAP: f32 = 6.0;
const SECTION_HEIGHT: f32 = 38.0;
const ROW_RADIUS: f32 = 14.0;

/// Comportement d'une ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Blur,
    Fps,
    /// Ligne informative : affiche un message, sans effet pour l'instant.
    Notice(&'static str),
    Refresh,
    Reconnect,
    Sleep,
    Shutdown,
    Quit,
}

/// Valeur affichée à droite d'une ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Value {
    None,
    Text(&'static str),
    GameCount,
    Connection,
    Profile,
}

#[derive(Debug, Clone, Copy)]
struct Row {
    /// Titre de rubrique affiché au-dessus de la ligne, s'il y en a un.
    section: Option<&'static str>,
    title: &'static str,
    subtitle: &'static str,
    kind: Kind,
    value: Value,
    danger: bool,
}

const ROWS: [Row; 10] = [
    Row {
        section: Some("INTERNET"),
        title: "Connexion réseau",
        subtitle: "Wi-Fi et configuration de l'appareil",
        kind: Kind::Notice("Configuration réseau bientôt disponible"),
        value: Value::Text("Non configuré"),
        danger: false,
    },
    Row {
        section: None,
        title: "Reconnecter le démon",
        subtitle: "Relance la connexion au socket Unix",
        kind: Kind::Reconnect,
        value: Value::Connection,
        danger: false,
    },
    Row {
        section: Some("MISE À JOUR"),
        title: "ZeedeFi OS",
        subtitle: "Version installée sur cet appareil",
        kind: Kind::Notice("Vérification des mises à jour bientôt disponible"),
        value: Value::Text("0.1.0"),
        danger: false,
    },
    Row {
        section: Some("COMPTE"),
        title: "Profil actif",
        subtitle: "Compte local de cet appareil",
        kind: Kind::Notice("Gestion des comptes bientôt disponible"),
        value: Value::Profile,
        danger: false,
    },
    Row {
        section: Some("BIBLIOTHÈQUE"),
        title: "Rafraîchir la bibliothèque",
        subtitle: "Redemande la liste des jeux au démon",
        kind: Kind::Refresh,
        value: Value::GameCount,
        danger: false,
    },
    Row {
        section: Some("AFFICHAGE"),
        title: "Flou d'arrière-plan",
        subtitle: "Adoucit l'artwork du jeu en arrière-plan",
        kind: Kind::Blur,
        value: Value::None,
        danger: false,
    },
    Row {
        section: None,
        title: "Afficher les FPS",
        subtitle: "Compteur de performance en haut à gauche",
        kind: Kind::Fps,
        value: Value::None,
        danger: false,
    },
    Row {
        section: Some("SYSTÈME"),
        title: "Mettre en veille",
        subtitle: "Délégué au démon ZeedeFi",
        kind: Kind::Sleep,
        value: Value::None,
        danger: false,
    },
    Row {
        section: None,
        title: "Éteindre",
        subtitle: "Arrêt complet de la machine (demande au démon)",
        kind: Kind::Shutdown,
        value: Value::None,
        danger: true,
    },
    Row {
        section: None,
        title: "Quitter le launcher",
        subtitle: "Ferme uniquement cette interface",
        kind: Kind::Quit,
        value: Value::None,
        danger: true,
    },
];

/// État de l'écran de paramètres.
#[derive(Debug, Clone)]
pub struct SettingsPanel {
    pub open: bool,
    anim: f32,
    /// Défilement vertical animé de la liste.
    scroll: f32,
    scrolling: bool,
    pub selected: usize,
    /// Rectangle de la ligne sélectionnée, ciblé par le sélecteur global.
    pub focus_rect: Option<Rect>,
    pub blur_enabled: bool,
    pub show_fps: bool,
    pub connection: ConnectionStatus,
    pub game_count: usize,
    pub error_code: Option<u32>,
    pub profile_name: String,
    /// Ligne dont l'action dangereuse attend une seconde validation.
    confirm: Option<usize>,
}

/// Action produite par l'écran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingAction {
    ToggleBlur,
    ToggleFps,
    RefreshLibrary,
    Reconnect,
    Sleep,
    Shutdown,
    Quit,
    /// Affiche un message d'information sans action système.
    Notice(&'static str),
    Close,
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self {
            open: false,
            anim: 0.0,
            scroll: 0.0,
            scrolling: false,
            selected: 0,
            focus_rect: None,
            blur_enabled: true,
            show_fps: false,
            connection: ConnectionStatus::Disconnected,
            game_count: 0,
            error_code: None,
            profile_name: String::new(),
            confirm: None,
        }
    }

    /// Ouvre l'écran.
    pub fn open_now(&mut self) {
        self.open = true;
        self.confirm = None;
    }

    pub fn close_now(&mut self) {
        self.open = false;
        self.confirm = None;
    }

    /// Vrai tant que l'écran est visible (ou en train de disparaître) : l'accueil
    /// est alors masqué.
    pub fn is_visible(&self) -> bool {
        self.open || self.anim > 0.0
    }

    /// Vrai tant qu'une transition ou un défilement est en cours (repaint).
    pub fn is_animating(&self) -> bool {
        let fading = if self.open {
            self.anim < 0.999
        } else {
            self.anim > 0.0
        };
        fading || self.scrolling
    }

    /// Gère une action d'entrée. Retourne une action sémantique si applicable.
    pub fn handle_action(&mut self, action: &Action) -> Option<SettingAction> {
        if !self.open {
            return None;
        }
        match action {
            Action::Navigate(Direction::Up) => {
                self.confirm = None;
                self.selected = (self.selected + ROWS.len() - 1) % ROWS.len();
                None
            }
            Action::Navigate(Direction::Down) => {
                self.confirm = None;
                self.selected = (self.selected + 1) % ROWS.len();
                None
            }
            Action::Navigate(Direction::Left | Direction::Right) => {
                // Gauche/droite basculent directement les interrupteurs.
                match ROWS[self.selected].kind {
                    Kind::Blur | Kind::Fps => self.activate(self.selected),
                    _ => None,
                }
            }
            Action::Confirm => self.activate(self.selected),
            Action::Back => Some(SettingAction::Close),
            _ => None,
        }
    }

    fn activate(&mut self, index: usize) -> Option<SettingAction> {
        let row = ROWS.get(index)?;
        match row.kind {
            Kind::Blur => Some(SettingAction::ToggleBlur),
            Kind::Fps => Some(SettingAction::ToggleFps),
            Kind::Notice(message) => Some(SettingAction::Notice(message)),
            Kind::Refresh => Some(SettingAction::RefreshLibrary),
            Kind::Reconnect => Some(SettingAction::Reconnect),
            Kind::Sleep => Some(SettingAction::Sleep),
            Kind::Shutdown | Kind::Quit => {
                // Action dangereuse : deux validations successives.
                if self.confirm == Some(index) {
                    self.confirm = None;
                    Some(if row.kind == Kind::Shutdown {
                        SettingAction::Shutdown
                    } else {
                        SettingAction::Quit
                    })
                } else {
                    self.confirm = Some(index);
                    None
                }
            }
        }
    }

    /// Dessine l'écran plein et retourne l'action demandée, s'il y en a une.
    pub fn draw(
        &mut self,
        context: &egui::Context,
        viewport: Rect,
        delta_seconds: f32,
    ) -> Option<SettingAction> {
        let target = if self.open { 1.0 } else { 0.0 };
        self.anim = theme::damp(self.anim, target, delta_seconds, 10.0);
        if !self.open && self.anim < 0.004 {
            self.anim = 0.0;
        }
        self.focus_rect = None;
        if !self.is_visible() {
            self.scrolling = false;
            return None;
        }

        let palette = theme::Palette::default();
        let e = theme::ease_out_cubic(self.anim);
        let painter = context.layer_painter(LayerId::new(Order::Middle, Id::new("settings_canvas")));

        // Canevas : l'accueil reste à peine visible derrière.
        painter.rect_filled(
            viewport,
            Rounding::ZERO,
            theme::with_alpha(palette.background, 0.92 * e),
        );

        let margin = (viewport.width() * 0.07).max(theme::CONTENT_MARGIN);
        let slide = (1.0 - e) * 24.0;
        let left = viewport.min.x + margin;
        let right = viewport.max.x - margin;

        // En-tête : même langage que l'accueil (filet, surtitre, titre).
        let overline_y = viewport.min.y + 96.0 + slide;
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(left, overline_y - 11.0), Vec2::new(3.0, 11.0)),
            Rounding::same(1.5),
            theme::with_alpha(theme::ACCENT, e),
        );
        painter.text(
            Pos2::new(left + 12.0, overline_y),
            egui::Align2::LEFT_BOTTOM,
            "PARAMÈTRES ZEEDEFI",
            theme::bold(11.0),
            theme::with_alpha(palette.text_muted, 0.95 * e),
        );
        painter.text(
            Pos2::new(left, overline_y + 54.0),
            egui::Align2::LEFT_BOTTOM,
            "Paramètres",
            theme::bold(44.0),
            theme::with_alpha(palette.text_primary, e),
        );

        // Liste verticale centrée, défilante.
        let list_top = (overline_y + 96.0).min(viewport.max.y - 160.0);
        let view = Rect::from_min_max(
            Pos2::new(left, list_top),
            Pos2::new(right, viewport.max.y - 40.0),
        );

        let mut tops = [0.0_f32; ROWS.len()];
        let mut cursor = 0.0_f32;
        for (index, row) in ROWS.iter().enumerate() {
            if row.section.is_some() {
                if index > 0 {
                    cursor += ROW_GAP * 2.0;
                }
                cursor += SECTION_HEIGHT;
            }
            tops[index] = cursor;
            cursor += ROW_HEIGHT + ROW_GAP;
        }
        let content_height = cursor;

        // Défilement : garder la ligne sélectionnée (et sa rubrique) visible.
        let selected = self.selected.min(ROWS.len() - 1);
        let section_space = if ROWS[selected].section.is_some() {
            SECTION_HEIGHT
        } else {
            0.0
        };
        let row_top = tops[selected] - section_space;
        let row_bottom = tops[selected] + ROW_HEIGHT;
        let view_height = view.height();
        let max_scroll = (content_height - view_height).max(0.0);
        let mut wanted = self.scroll;
        if row_top < wanted {
            wanted = row_top;
        }
        if row_bottom > wanted + view_height {
            wanted = row_bottom - view_height;
        }
        let wanted = wanted.clamp(0.0, max_scroll);
        self.scroll = theme::damp(self.scroll, wanted, delta_seconds, 12.0);
        self.scrolling = (self.scroll - wanted).abs() > 0.3;

        let list_painter = painter.with_clip_rect(view);
        for (index, row) in ROWS.iter().enumerate() {
            let top = view.min.y + tops[index] - self.scroll;
            if let Some(section) = row.section {
                list_painter.text(
                    Pos2::new(left + 4.0, top - SECTION_HEIGHT + 10.0),
                    egui::Align2::LEFT_TOP,
                    section,
                    theme::bold(10.5),
                    theme::with_alpha(palette.text_muted, 0.95 * e),
                );
            }

            let rect = Rect::from_min_size(
                Pos2::new(left, top),
                Vec2::new(right - left, ROW_HEIGHT),
            );
            if rect.max.y < view.min.y || rect.min.y > view.max.y {
                continue;
            }

            let toggle = match row.kind {
                Kind::Blur => Some(self.blur_enabled),
                Kind::Fps => Some(self.show_fps),
                _ => None,
            };
            let confirming = self.confirm == Some(index);
            let value = self.value_text(row.value);
            paint_row(&list_painter, rect, index, row, value, toggle, confirming, e);

            if index == selected {
                self.focus_rect = Some(rect.intersect(view));
            }
        }

        None
    }

    fn value_text(&self, value: Value) -> Option<String> {
        match value {
            Value::None => None,
            Value::Text(text) => Some(text.to_string()),
            Value::GameCount => Some(format!("{} jeux", self.game_count)),
            Value::Profile => Some(self.profile_name.clone()),
            Value::Connection => Some(match &self.connection {
                ConnectionStatus::Connected => "Connecté".to_string(),
                ConnectionStatus::Connecting => "Connexion…".to_string(),
                ConnectionStatus::Disconnected => "Hors ligne".to_string(),
                ConnectionStatus::Failed { code } => format!("Erreur {code}"),
            }),
        }
    }
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Dessine une ligne : titre, sous-titre, puis interrupteur ou valeur à droite.
#[allow(clippy::too_many_arguments)]
fn paint_row(
    painter: &egui::Painter,
    rect: Rect,
    index: usize,
    row: &Row,
    value: Option<String>,
    toggle: Option<bool>,
    confirming: bool,
    e: f32,
) {
    let palette = theme::Palette::default();
    let rounding = Rounding::same(ROW_RADIUS);
    painter.rect_filled(rect, rounding, theme::with_alpha(Color32::WHITE, 0.05 * e));
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.06 * e)),
    );

    let title_color = if row.danger {
        palette.danger
    } else {
        palette.text_primary
    };
    painter.text(
        Pos2::new(rect.min.x + 28.0, rect.center().y - 11.0),
        egui::Align2::LEFT_CENTER,
        row.title,
        theme::bold(18.0),
        theme::with_alpha(title_color, e),
    );

    let (subtitle, subtitle_color) = if confirming {
        ("Appuyez à nouveau sur A pour confirmer", palette.danger)
    } else {
        (row.subtitle, palette.text_muted)
    };
    painter.text(
        Pos2::new(rect.min.x + 28.0, rect.center().y + 12.0),
        egui::Align2::LEFT_CENTER,
        subtitle,
        theme::font(13.0),
        theme::with_alpha(subtitle_color, e),
    );

    if let Some(on) = toggle {
        let t = painter
            .ctx()
            .animate_bool(Id::new(("settings_toggle", index)), on);
        draw_toggle(painter, Pos2::new(rect.max.x - 57.0, rect.center().y), t);
    } else if let Some(text) = value {
        painter.text(
            Pos2::new(rect.max.x - 36.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            text,
            theme::font(15.0),
            theme::with_alpha(palette.text_secondary, e),
        );
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
