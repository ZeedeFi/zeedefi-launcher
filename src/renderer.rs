//! `src/renderer.rs` — rendu du fond d'écran (image plein écran, overlay sombre,
//! dégradé de repli, fondu lors du changement de jeu focus).
//!
//! L'ordre de rendu respecté est : fond → overlay sombre → UI egui. Les
//! shapes sont déposées dans la couche `Background` d'egui, qui est peinte
//! avant les panneaux et les widgets.

use std::path::PathBuf;

use egui::{Color32, Context, Painter, Pos2, Rect, TextureHandle, Vec2};

use crate::theme;

#[derive(Clone)]
struct BackgroundLayer {
    handle: TextureHandle,
    path: PathBuf,
}

impl std::fmt::Debug for BackgroundLayer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BackgroundLayer")
            .field("path", &self.path)
            .finish()
    }
}

/// Gestionnaire du fond d'écran.
#[derive(Debug, Default)]
pub struct BackgroundRenderer {
    current: Option<BackgroundLayer>,
    previous: Option<BackgroundLayer>,
    pub fade: f32,
    overlay_alpha: f32,
}

impl BackgroundRenderer {
    pub fn new() -> Self {
        Self {
            current: None,
            previous: None,
            fade: 1.0,
            overlay_alpha: theme::OVERLAY_ALPHA,
        }
    }

    /// Définit le fond d'écran courant. Si le chemin change, l'ancien fond est
    /// conservé pour un fondu sortant.
    pub fn set_background(&mut self, handle: TextureHandle, path: PathBuf) {
        let already_current = self
            .current
            .as_ref()
            .map(|layer| layer.path == path && layer.handle.id() == handle.id())
            .unwrap_or(false);
        if already_current {
            return;
        }
        self.previous = self.current.take();
        self.current = Some(BackgroundLayer { handle, path });
        self.fade = 0.0;
    }

    /// Efface le fond courant (utilisé en mode hors-ligne sans illustration).
    pub fn clear(&mut self) {
        if self.current.is_none() && self.previous.is_none() {
            return;
        }
        self.previous = self.current.take();
        self.fade = 0.0;
    }

    /// Avance l'animation du fondu.
    pub fn update_animation(&mut self, delta_seconds: f32) {
        if self.fade < 1.0 {
            self.fade = (self.fade + delta_seconds * 4.0).min(1.0);
        }
    }

    /// Peint le fond d'écran dans la couche Background d'egui.
    pub fn draw(&self, context: &Context, viewport: Rect, fallback_id: Option<&str>) {
        let painter = context.layer_painter(egui::LayerId::background());

        // Fond de repli (dégradé ou couleur unie).
        let palette = theme::Palette::default();
        if let Some(id) = fallback_id {
            let colors = theme::fallback_gradient(id);
            paint_vertical_gradient(&painter, viewport, colors[0], colors[1]);
        } else {
            painter.rect_filled(viewport, egui::Rounding::ZERO, palette.background);
        }

        // Couche précédente (sortante).
        if let Some(previous) = &self.previous {
            let alpha = 1.0 - self.fade;
            draw_layer(&painter, previous, viewport, alpha);
        }

        // Couche courante (entrante).
        if let Some(current) = &self.current {
            let alpha = if self.previous.is_some() {
                self.fade
            } else {
                1.0
            };
            draw_layer(&painter, current, viewport, alpha);
        }

        // Halo ambiant lorsque aucun artwork n'est disponible (mode démo).
        if self.current.is_none() {
            if let Some(id) = fallback_id {
                let colors = theme::fallback_gradient(id);
                paint_ambient_glow(&painter, viewport, colors[0]);
            }
        }

        // Overlay sombre pour la lisibilité du texte.
        painter.rect_filled(
            viewport,
            egui::Rounding::ZERO,
            theme::with_alpha(Color32::BLACK, self.overlay_alpha),
        );

        // Voile supérieur : lisibilité du profil et de la pilule d'état.
        let top_height = 190.0_f32.min(viewport.height());
        let top = Rect::from_min_size(
            Pos2::new(viewport.min.x, viewport.min.y),
            Vec2::new(viewport.width(), top_height),
        );
        paint_vertical_gradient(
            &painter,
            top,
            theme::with_alpha(Color32::BLACK, 0.20),
            theme::with_alpha(Color32::BLACK, 0.0),
        );

        // Voile latéral gauche : lisibilité du bloc d'information.
        let left_width = (viewport.width() * 0.42).min(820.0);
        let left = Rect::from_min_size(
            Pos2::new(viewport.min.x, viewport.min.y),
            Vec2::new(left_width, viewport.height()),
        );
        draw_horizontal_fade(&painter, left, 0.14);

        // Dégradé supplémentaire en bas pour le carrousel.
        let bottom_height = 220.0_f32.min(viewport.height());
        let bottom = Rect::from_min_size(
            Pos2::new(viewport.min.x, viewport.max.y - bottom_height),
            Vec2::new(viewport.width(), bottom_height),
        );
        paint_vertical_gradient(
            &painter,
            bottom,
            theme::with_alpha(Color32::BLACK, 0.0),
            theme::with_alpha(Color32::BLACK, 0.55),
        );
    }
}

fn draw_layer(painter: &Painter, layer: &BackgroundLayer, viewport: Rect, alpha: f32) {
    if alpha <= 0.0 {
        return;
    }
    let size = layer.handle.size();
    if size[0] == 0 || size[1] == 0 {
        return;
    }
    let scale = (viewport.width() / size[0] as f32)
        .max(viewport.height() / size[1] as f32);
    let width = size[0] as f32 * scale;
    let height = size[1] as f32 * scale;
    let rect = Rect::from_center_size(viewport.center(), Vec2::new(width, height));
    let tint = theme::with_alpha(Color32::WHITE, alpha);
    painter.image(
        layer.handle.id(),
        rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        tint,
    );
}

fn paint_vertical_gradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    // egui ne fournit pas de helper de dégradé simple dans toutes les versions ;
    // on approxime par une série de bandes horizontales (coût négligeable).
    let steps = 32usize;
    let height = rect.height();
    if height <= 0.0 {
        return;
    }
    for step in 0..steps {
        let t0 = step as f32 / steps as f32;
        let t1 = (step + 1) as f32 / steps as f32;
        let y0 = rect.min.y + t0 * height;
        let y1 = rect.min.y + t1 * height;
        let color = lerp_color(top, bottom, (t0 + t1) * 0.5);
        let band = Rect::from_min_max(
            Pos2::new(rect.min.x, y0),
            Pos2::new(rect.max.x, y1 + 1.0),
        );
        painter.rect_filled(band, egui::Rounding::ZERO, color);
    }
}

fn paint_ambient_glow(painter: &Painter, viewport: Rect, color: Color32) {
    let center = Pos2::new(
        viewport.min.x + viewport.width() * 0.66,
        viewport.min.y + viewport.height() * 0.40,
    );
    let max_radius = viewport.width() * 0.58;
    let glow = theme::lerp_color(color, Color32::WHITE, 0.35);
    let layers = 30;
    for step in 0..layers {
        let t = step as f32 / layers as f32;
        let radius = max_radius * (1.0 - t * 0.92);
        painter.circle_filled(
            center,
            radius,
            theme::with_alpha(glow, 0.012),
        );
    }
}

fn draw_horizontal_fade(painter: &Painter, rect: Rect, alpha: f32) {
    let steps = 24usize;
    let width = rect.width();
    if width <= 0.0 {
        return;
    }
    for step in 0..steps {
        let t0 = step as f32 / steps as f32;
        let t1 = (step + 1) as f32 / steps as f32;
        let band = Rect::from_min_max(
            Pos2::new(rect.min.x + t0 * width, rect.min.y),
            Pos2::new(rect.min.x + t1 * width + 1.0, rect.max.y),
        );
        let a = alpha * (1.0 - (t0 + t1) * 0.5).powi(2);
        painter.rect_filled(
            band,
            egui::Rounding::ZERO,
            theme::with_alpha(Color32::BLACK, a),
        );
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let channel = |from: u8, to: u8| {
        (f32::from(from) + (f32::from(to) - f32::from(from)) * t).round() as u8
    };
    Color32::from_rgba_unmultiplied(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
        channel(a.a(), b.a()),
    )
}
