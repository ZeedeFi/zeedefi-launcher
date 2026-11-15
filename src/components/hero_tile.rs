//! `src/components/hero_tile.rs` — bloc d'information du jeu sélectionné.
//!
//! Le bloc est aligné au-dessus de la grande tuile : surtitre discret, titre
//! principal, sous-titre puis pastilles d'action (Jouer / Détails / réglages).
//! L'interface est pilotée exclusivement au clavier ou à la manette : les
//! pastilles sont peintes, jamais cliquables, et leur état reflète
//! `hero_action`.

use egui::{Color32, Pos2, Rect, Rounding, Stroke, Ui, Vec2};

use crate::app::GameEntry;
use crate::components::icons;
use crate::theme;

const PILL_HEIGHT: f32 = 46.0;
const PILL_GAP: f32 = 12.0;

/// Dessine le bloc d'information du jeu sélectionné.
pub fn draw(ui: &Ui, area: Rect, entry: &GameEntry, hero_action: usize) {
    let painter = ui.painter_at(area);

    let x = area.min.x + theme::CONTENT_MARGIN;
    let max_text_width = (area.max.x - theme::CONTENT_MARGIN - x).max(120.0);

    let pills_top = area.max.y - 18.0 - PILL_HEIGHT;
    let subtitle_bottom = pills_top - 20.0;
    let title_bottom = subtitle_bottom - 22.0;
    let overline_bottom = title_bottom - 58.0;

    // Surtitre.
    painter.text(
        Pos2::new(x, overline_bottom),
        egui::Align2::LEFT_BOTTOM,
        "BIBLIOTHÈQUE ZEEDEFI",
        theme::bold(11.0),
        theme::with_alpha(theme::Palette::default().text_muted, 0.95),
    );

    // Titre principal.
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

    // --- Pastille primaire « Jouer » ---
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
    paint_primary_pill(&painter, play_rect, hero_action == 0);

    // --- Pastille secondaire « Détails » ---
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
    paint_secondary_pill(&painter, details_rect, hero_action == 1);

    // --- Bouton rond « Paramètres » ---
    let settings_size = PILL_HEIGHT;
    let settings_rect = Rect::from_min_size(
        Pos2::new(details_rect.max.x + PILL_GAP, pills_top),
        Vec2::splat(settings_size),
    );
    paint_icon_button(&painter, settings_rect, hero_action == 2);
    icons::sliders(
        &painter,
        settings_rect.shrink(settings_size * 0.31),
        theme::Palette::default().text_primary,
    );
}

fn paint_primary_pill(painter: &egui::Painter, rect: Rect, selected: bool) {
    let rounding = Rounding::same(rect.height() * 0.5);
    let fill = if selected {
        theme::ACCENT
    } else {
        theme::with_alpha(theme::ACCENT, 0.88)
    };
    painter.rect_filled(rect, rounding, fill);

    if selected {
        for layer in 1..=3 {
            painter.rect_stroke(
                rect.expand(layer as f32 * 3.0),
                rounding,
                Stroke::new(
                    1.2_f32,
                    theme::with_alpha(Color32::WHITE, 0.26 / layer as f32),
                ),
            );
        }
        painter.rect_stroke(
            rect.expand(2.0),
            rounding,
            Stroke::new(1.5_f32, theme::with_alpha(Color32::WHITE, 0.85)),
        );
    }

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

fn paint_secondary_pill(painter: &egui::Painter, rect: Rect, selected: bool) {
    let rounding = Rounding::same(rect.height() * 0.5);
    painter.rect_filled(
        rect,
        rounding,
        theme::with_alpha(Color32::WHITE, if selected { 0.15 } else { 0.10 }),
    );
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(
            1.0_f32,
            theme::with_alpha(Color32::WHITE, if selected { 0.7 } else { 0.16 }),
        ),
    );
    if selected {
        painter.rect_stroke(
            rect.expand(3.0),
            rounding,
            Stroke::new(1.2_f32, theme::with_alpha(theme::ACCENT, 0.55)),
        );
    }

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

fn paint_icon_button(painter: &egui::Painter, rect: Rect, selected: bool) {
    let rounding = Rounding::same(rect.height() * 0.5);
    painter.rect_filled(
        rect,
        rounding,
        theme::with_alpha(Color32::WHITE, if selected { 0.15 } else { 0.10 }),
    );
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(
            1.0_f32,
            theme::with_alpha(Color32::WHITE, if selected { 0.7 } else { 0.16 }),
        ),
    );
    if selected {
        painter.rect_stroke(
            rect.expand(3.0),
            rounding,
            Stroke::new(1.2_f32, theme::with_alpha(theme::ACCENT, 0.55)),
        );
    }
}
