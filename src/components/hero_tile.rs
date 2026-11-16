//! `src/components/hero_tile.rs` — bloc d'information du jeu sélectionné.
//!
//! Le bloc est aligné au-dessus de la grande tuile : surtitre discret, titre
//! principal, sous-titre puis trois actions (Jouer / Détails / réglages).
//! Les actions sont peintes sans état de focus : le sélecteur bleu unique est
//! dessiné par `app.rs` à partir des rectangles retournés par `draw`.
//!
//! La transparence globale du bloc est appliquée par l'appelant via
//! `Ui::set_opacity` ; `enter` (0 → 1) fait monter le bloc en place.

use egui::{Color32, Pos2, Rect, Rounding, Stroke, Ui, Vec2};

use crate::app::GameEntry;
use crate::components::icons;
use crate::theme;

const PILL_HEIGHT: f32 = 46.0;
const PILL_GAP: f32 = 12.0;
/// Distance parcourue par le bloc pendant son entrée (en pixels).
const RISE_DISTANCE: f32 = 28.0;

/// Dessine le bloc et retourne les rectangles des trois actions
/// (Jouer, Détails, réglages), dans l'ordre d'affichage.
pub fn draw(ui: &Ui, area: Rect, entry: &GameEntry, enter: f32) -> [Rect; 3] {
    // Le clip est élargi pour que le bloc qui monte ne soit jamais coupé.
    let painter = ui.painter_at(area.expand2(Vec2::new(0.0, RISE_DISTANCE + 20.0)));
    let area = area.translate(Vec2::new(
        0.0,
        (1.0 - enter.clamp(0.0, 1.0)) * RISE_DISTANCE,
    ));

    let x = area.min.x + theme::CONTENT_MARGIN;
    let max_text_width = (area.max.x - theme::CONTENT_MARGIN - x).max(120.0);

    let pills_top = area.max.y - 28.0 - PILL_HEIGHT;
    let subtitle_bottom = pills_top - 20.0;
    let title_bottom = subtitle_bottom - 22.0;
    let overline_bottom = title_bottom - 58.0;

    // Surtitre précédé d'un filet accent.
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(x, overline_bottom - 11.0),
            Vec2::new(3.0, 11.0),
        ),
        Rounding::same(1.5),
        theme::ACCENT,
    );
    painter.text(
        Pos2::new(x + 12.0, overline_bottom),
        egui::Align2::LEFT_BOTTOM,
        "BIBLIOTHÈQUE ZEEDEFI",
        theme::bold(11.0),
        theme::with_alpha(theme::Palette::default().text_muted, 0.95),
    );

    // Titre principal, précédé d'une ombre portée douce pour la lisibilité
    // sur l'artwork.
    theme::ellipsis_text(
        &painter,
        Pos2::new(x, title_bottom + 3.0),
        egui::Align2::LEFT_BOTTOM,
        &entry.title,
        theme::bold(44.0),
        theme::with_alpha(Color32::BLACK, 0.45),
        max_text_width,
    );
    theme::ellipsis_text(
        &painter,
        Pos2::new(x, title_bottom),
        egui::Align2::LEFT_BOTTOM,
        &entry.title,
        theme::bold(44.0),
        theme::Palette::default().text_primary,
        max_text_width,
    );

    // Sous-titre : pastille de statut + métadonnées.
    let palette = theme::Palette::default();
    let dot = Pos2::new(x + 4.0, subtitle_bottom - 7.0);
    painter.circle_filled(dot, 3.0, palette.accent);
    painter.text(
        Pos2::new(x + 16.0, subtitle_bottom),
        egui::Align2::LEFT_BOTTOM,
        "Installé  ·  ZeedeFi OS",
        theme::font(15.0),
        palette.text_secondary,
    );

    // --- Action « Jouer » ---
    let play_label = "Jouer";
    let play_text_width = painter
        .layout_no_wrap(play_label.to_string(), theme::bold(16.0), Color32::WHITE)
        .size()
        .x;
    let play_width = 24.0 + 16.0 + 10.0 + play_text_width + 24.0;
    let play_rect = Rect::from_min_size(
        Pos2::new(x, pills_top),
        Vec2::new(play_width, PILL_HEIGHT),
    );
    paint_primary_pill(&painter, play_rect);

    // --- Action « Détails » ---
    let details_label = "Détails";
    let details_text_width = painter
        .layout_no_wrap(
            details_label.to_string(),
            theme::bold(15.0),
            Color32::WHITE,
        )
        .size()
        .x;
    let details_width = 22.0 + 16.0 + 9.0 + details_text_width + 22.0;
    let details_rect = Rect::from_min_size(
        Pos2::new(play_rect.max.x + PILL_GAP, pills_top),
        Vec2::new(details_width, PILL_HEIGHT),
    );
    paint_secondary_pill(&painter, details_rect);

    // --- Action « Paramètres » (bouton rond) ---
    let settings_rect = Rect::from_min_size(
        Pos2::new(details_rect.max.x + PILL_GAP, pills_top),
        Vec2::splat(PILL_HEIGHT),
    );
    paint_icon_button(&painter, settings_rect);
    icons::sliders(
        &painter,
        settings_rect.shrink(PILL_HEIGHT * 0.31),
        palette.text_primary,
    );

    [play_rect, details_rect, settings_rect]
}

fn paint_primary_pill(painter: &egui::Painter, rect: Rect) {
    let rounding = Rounding::same(rect.height() * 0.5);
    painter.rect_filled(rect, rounding, theme::ACCENT);

    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.min.x + 24.0 + 8.0, rect.center().y),
        Vec2::splat(16.0),
    );
    icons::play(painter, icon_rect, Color32::from_rgb(0x06, 0x13, 0x1F));
    painter.text(
        Pos2::new(icon_rect.max.x + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "Jouer",
        theme::bold(16.0),
        Color32::from_rgb(0x06, 0x13, 0x1F),
    );
}

fn paint_secondary_pill(painter: &egui::Painter, rect: Rect) {
    let rounding = Rounding::same(rect.height() * 0.5);
    painter.rect_filled(rect, rounding, theme::with_alpha(Color32::WHITE, 0.10));
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.16)),
    );

    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.min.x + 22.0 + 8.0, rect.center().y),
        Vec2::splat(16.0),
    );
    icons::info(painter, icon_rect, theme::Palette::default().text_primary);
    painter.text(
        Pos2::new(icon_rect.max.x + 9.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "Détails",
        theme::bold(15.0),
        theme::Palette::default().text_primary,
    );
}

fn paint_icon_button(painter: &egui::Painter, rect: Rect) {
    let rounding = Rounding::same(rect.height() * 0.5);
    painter.rect_filled(rect, rounding, theme::with_alpha(Color32::WHITE, 0.10));
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.16)),
    );
}
