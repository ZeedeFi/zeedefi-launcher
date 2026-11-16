//! `src/components/carousel.rs` — rangée de tuiles carrées, style Xbox Series X.
//!
//! La tuile sélectionnée est agrandie et ancrée à gauche ; les tuiles suivantes
//! s'enchaînent horizontalement. Les positions et les échelles sont animées par
//! `AppState` (interpolation indépendante du framerate), le rendu se contente
//! de peindre l'état courant. Navigation exclusivement clavier/manette : aucune
//! tuile n'est cliquable, le focus est unique et toujours visible.

use egui::{Align2, Color32, Painter, Pos2, Rect, Rounding, Sense, Stroke, TextureHandle, Ui, Vec2};

use crate::app::AppState;
use crate::components::game_tile::{self, TileStyle, TileVisual};
use crate::theme;

/// Textures disponibles pour les tuiles (dans l'ordre de la bibliothèque).
pub struct TileVisuals<'a> {
    pub covers: &'a [Option<TextureHandle>],
    pub gradients: &'a [TextureHandle],
}

/// Dessine la rangée de tuiles.
pub fn draw(ui: &mut Ui, area: Rect, state: &AppState, visuals: &TileVisuals<'_>) -> Option<Rect> {
    let mut focused_rect = None;
    let _ = ui.allocate_exact_size(area.size(), Sense::hover());
    let row_bottom = area.max.y - theme::ROW_BOTTOM_PADDING;

    // Sol lumineux très discret sous la rangée : ancre les tuiles dans l'image.
    let floor = Rect::from_min_max(
        egui::Pos2::new(area.min.x, row_bottom - 26.0),
        egui::Pos2::new(area.max.x, row_bottom + 54.0),
    );
    theme::vertical_gradient(
        ui.painter(),
        floor,
        theme::with_alpha(theme::ACCENT, 0.0),
        theme::with_alpha(theme::ACCENT, 0.045),
    );

    // Deux passes : les tuiles secondaires d'abord, la tuile focus au-dessus.
    for pass_focused in [false, true] {
        for (index, entry) in state.games_library.iter().enumerate() {
            let focused = index == state.focused_game_index;
            if focused != pass_focused {
                continue;
            }

            let alpha = state.tile_alpha.get(index).copied().unwrap_or(1.0);
            if alpha <= 0.01 {
                continue;
            }
            let scale = state
                .tile_scales
                .get(index)
                .copied()
                .unwrap_or(1.0)
                .max(0.05);
            let x = state
                .tile_x
                .get(index)
                .copied()
                .unwrap_or(area.min.x + theme::CONTENT_MARGIN);
            let size = theme::TILE_SIZE * scale;
            let rect = Rect::from_min_max(
                egui::Pos2::new(x, row_bottom - size.y),
                egui::Pos2::new(x + size.x, row_bottom),
            );

            if focused {
                focused_rect = Some(rect);
            }

            // Culling horizontal large (les tuiles qui glissent restent fluides).
            if rect.max.x < area.min.x - 24.0 || rect.min.x > area.max.x + 24.0 {
                continue;
            }

            let cover = visuals.covers.get(index).and_then(|handle| handle.as_ref());
            let gradient = match visuals.gradients.get(index) {
                Some(texture) => texture,
                None => continue,
            };

            game_tile::draw(
                ui,
                rect,
                TileVisual {
                    entry,
                    cover,
                    gradient,
                },
                TileStyle {
                    focused,
                    alpha,
                    shine: if focused { state.focus_shine } else { 1.0 },
                },
            );
        }
    }
    paint_footer(ui, area, row_bottom, state);
    focused_rect
}

/// Pied de rangée : position dans la bibliothèque (à gauche) et aide des
/// touches (à droite), dans l'espace réservé sous les tuiles.
fn paint_footer(ui: &Ui, area: Rect, row_bottom: f32, state: &AppState) {
    let count = state.games_library.len();
    if count == 0 {
        return;
    }
    let painter = ui.painter();
    let palette = theme::Palette::default();
    let left = area.min.x + theme::CONTENT_MARGIN;
    let center_y = row_bottom + 30.0;

    let current = state.focused_game_index.min(count - 1) + 1;
    painter.text(
        Pos2::new(left, center_y),
        Align2::LEFT_CENTER,
        format!("{current:02} / {count:02}"),
        theme::bold(12.0),
        theme::with_alpha(palette.text_secondary, 0.9),
    );

    let track = Rect::from_min_size(Pos2::new(left, center_y + 14.0), Vec2::new(120.0, 3.0));
    painter.rect_filled(track, Rounding::same(1.5), theme::with_alpha(Color32::WHITE, 0.14));
    let fraction = ui.ctx().animate_value_with_time(
        egui::Id::new("carousel_footer_progress"),
        current as f32 / count as f32,
        0.18,
    );
    painter.rect_filled(
        Rect::from_min_size(track.min, Vec2::new(track.width() * fraction, track.height())),
        Rounding::same(1.5),
        theme::ACCENT,
    );

    // Aide des touches, de droite à gauche.
    let mut right = area.max.x - theme::CONTENT_MARGIN;
    for (key, label) in [
        ("Start", "Profil"),
        ("Y", "Paramètres"),
        ("X", "Détails"),
        ("A", "Lancer"),
    ] {
        right = paint_hint(painter, right, center_y, key, label);
    }
}

/// Une pastille de touche suivie de son libellé, alignés à droite sur `right`.
/// Retourne l'abscisse où commence le groupe (avec un espace avant le suivant).
fn paint_hint(painter: &Painter, right: f32, center_y: f32, key: &str, label: &str) -> f32 {
    let palette = theme::Palette::default();
    let label_width = painter
        .layout_no_wrap(label.to_string(), theme::font(12.0), palette.text_muted)
        .size()
        .x;
    let key_width = painter
        .layout_no_wrap(key.to_string(), theme::bold(10.5), palette.text_secondary)
        .size()
        .x;
    let cap_width = (key_width + 14.0).max(20.0);

    painter.text(
        Pos2::new(right, center_y),
        Align2::RIGHT_CENTER,
        label,
        theme::font(12.0),
        palette.text_muted,
    );
    let cap = Rect::from_center_size(
        Pos2::new(right - label_width - 10.0 - cap_width * 0.5, center_y),
        Vec2::new(cap_width, 20.0),
    );
    painter.rect_filled(cap, Rounding::same(10.0), theme::with_alpha(Color32::WHITE, 0.10));
    painter.rect_stroke(
        cap,
        Rounding::same(10.0),
        Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.16)),
    );
    painter.text(
        cap.center(),
        Align2::CENTER_CENTER,
        key,
        theme::bold(10.5),
        palette.text_secondary,
    );
    cap.min.x - 26.0
}
