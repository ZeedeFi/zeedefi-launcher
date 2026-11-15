//! `src/components/game_tile.rs` — tuile carrée individuelle du carrousel.
//!
//! Reprend les codes de la Xbox Series X : artwork carré, coins arrondis,
//! ombre portée, anneau blanc et halo sur la tuile sélectionnée.

use egui::{Color32, Rect, Rounding, Stroke, TextureHandle, Ui, Vec2};

use crate::app::GameEntry;
use crate::theme;

/// Visuels nécessaires au dessin d'une tuile.
pub struct TileVisual<'a> {
    pub entry: &'a GameEntry,
    pub cover: Option<&'a TextureHandle>,
    pub gradient: &'a TextureHandle,
}

/// État visuel d'une tuile.
#[derive(Debug, Clone, Copy)]
pub struct TileStyle {
    pub focused: bool,
    pub alpha: f32,
}

/// Dessine une tuile carrée.
pub fn draw(ui: &Ui, rect: Rect, visual: TileVisual<'_>, style: TileStyle) {
    let alpha = style.alpha.clamp(0.0, 1.0);
    if alpha <= 0.01 || rect.width() <= 1.0 {
        return;
    }

    let painter = ui.painter();
    let rounding: Rounding = if style.focused {
        theme::focus_radius()
    } else {
        theme::tile_radius()
    };

    // Ombre portée : plus marquée sur la tuile focus.
    theme::soft_shadow(
        painter,
        rect,
        rounding,
        if style.focused { 1.0 } else { 0.55 } * alpha,
    );

    // Halo doux autour de la tuile sélectionnée.
    if style.focused {
        for layer in 1..=4 {
            let expand = layer as f32 * 3.0;
            painter.rect_filled(
                rect.expand(expand),
                rounding,
                theme::with_alpha(Color32::WHITE, alpha * 0.055 / layer as f32),
            );
        }
    }

    // Artwork (couverture) ou dégradé procédural, toujours arrondi.
    let texture = visual.cover.unwrap_or(visual.gradient);
    egui::Image::new(egui::load::SizedTexture::new(texture.id(), rect.size()))
        .rounding(rounding)
        .tint(theme::with_alpha(Color32::WHITE, alpha))
        .paint_at(ui, rect);

    if visual.cover.is_none() {
        // Léger voile pour asseoir les initiales sur le dégradé.
        theme::vertical_gradient(
            painter,
            rect,
            theme::with_alpha(Color32::BLACK, 0.0),
            theme::with_alpha(Color32::BLACK, 0.28 * alpha),
        );
        let initials = theme::initials(&visual.entry.title);
        let (font, color) = if style.focused {
            (
                theme::bold(84.0),
                theme::with_alpha(Color32::WHITE, alpha * 0.94),
            )
        } else {
            (
                theme::bold(30.0),
                theme::with_alpha(Color32::WHITE, alpha * 0.80),
            )
        };
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            initials,
            font,
            color,
        );
    }

    // Reflet supérieur très discret, façon verre.
    let sheen_height = (rect.height() * 0.35).max(12.0);
    let sheen = Rect::from_min_size(rect.min, Vec2::new(rect.width(), sheen_height));
    theme::vertical_gradient(
        painter,
        sheen,
        theme::with_alpha(Color32::WHITE, alpha * 0.06),
        theme::with_alpha(Color32::WHITE, 0.0),
    );

    // Bordure : anneau blanc net sur la sélection, liseré discret sinon.
    if style.focused {
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(2.5_f32, theme::with_alpha(Color32::WHITE, alpha * 0.92)),
        );
        painter.rect_stroke(
            rect.expand(5.0),
            rounding,
            Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, alpha * 0.20)),
        );
    } else {
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, alpha * 0.10)),
        );
    }
}
