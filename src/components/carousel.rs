//! `src/components/carousel.rs` — rangée de tuiles carrées, style Xbox Series X.
//!
//! La tuile sélectionnée est agrandie et ancrée à gauche ; les tuiles suivantes
//! s'enchaînent horizontalement. Les positions et les échelles sont animées par
//! `AppState` (interpolation indépendante du framerate), le rendu se contente
//! de peindre l'état courant.

use egui::{Id, Rect, Sense, TextureHandle, Ui};

use crate::app::AppState;
use crate::components::game_tile::{self, TileStyle, TileVisual};
use crate::theme;

/// Textures disponibles pour les tuiles (dans l'ordre de la bibliothèque).
pub struct TileVisuals<'a> {
    pub covers: &'a [Option<TextureHandle>],
    pub gradients: &'a [TextureHandle],
}

/// Dessine la rangée et retourne l'indice de la tuile cliquée, le cas échéant.
pub fn draw(
    ui: &mut Ui,
    area: Rect,
    state: &AppState,
    visuals: &TileVisuals<'_>,
) -> Option<usize> {
    let _ = ui.allocate_exact_size(area.size(), Sense::hover());
    let row_bottom = area.max.y - theme::ROW_BOTTOM_PADDING;
    let mut clicked = None;

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

            // Culling horizontal large (les tuiles qui glissent restent fluides).
            if rect.max.x < area.min.x - 24.0 || rect.min.x > area.max.x + 24.0 {
                continue;
            }

            // Interaction : seule la tuile réellement visible est cliquable.
            let interactive = alpha > 0.35 && rect.min.x < area.max.x - 8.0;
            let mut hovered = false;
            if interactive {
                let response = ui
                    .interact(rect, Id::new(("carousel_tile", index)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                hovered = response.hovered();
                if response.clicked() {
                    clicked = Some(index);
                }
                if !focused {
                    response.on_hover_text_at_pointer(entry.title.clone());
                }
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
                    hovered,
                    alpha,
                },
            );
        }
    }

    clicked
}
