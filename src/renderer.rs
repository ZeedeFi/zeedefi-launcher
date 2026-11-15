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
    /// Léger travelling avant au changement de fond (effet « push-in »).
    zoom: f32,
}

impl BackgroundRenderer {
    pub fn new() -> Self {
        Self {
            current: None,
            previous: None,
            fade: 1.0,
            overlay_alpha: theme::OVERLAY_ALPHA,
            zoom: 0.0,
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
        self.zoom = 0.045;
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
            self.fade = (self.fade + delta_seconds * 3.0).min(1.0);
        }
        // Le travelling se stabilise en douceur, indépendamment du framerate.
        self.zoom *= (-delta_seconds * 3.5).exp();
        if self.zoom < 0.0005 {
            self.zoom = 0.0;
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

        let fade = smoothstep(self.fade);
        let zoom = 1.0 + self.zoom;

        // Couche précédente (sortante).
        if let Some(previous) = &self.previous {
            draw_layer(&painter, previous, viewport, 1.0 - fade, zoom);
        }

        // Couche courante (entrante).
        if let Some(current) = &self.current {
            let alpha = if self.previous.is_some() { fade } else { 1.0 };
            draw_layer(&painter, current, viewport, alpha, zoom);
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

        // Vignettage doux : concentre le regard vers le centre de l'écran.
        paint_vignette(&painter, viewport, 0.14);

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

fn draw_layer(
    painter: &Painter,
    layer: &BackgroundLayer,
    viewport: Rect,
    alpha: f32,
    zoom: f32,
) {
    if alpha <= 0.0 {
        return;
    }
    let size = layer.handle.size();
    if size[0] == 0 || size[1] == 0 {
        return;
    }
    let scale = (viewport.width() / size[0] as f32)
        .max(viewport.height() / size[1] as f32)
        * zoom;
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

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn paint_vignette(painter: &Painter, viewport: Rect, strength: f32) {
    let center = viewport.center();
    let radius = (viewport.width().powi(2) + viewport.height().powi(2)).sqrt() * 0.5;
    let mut mesh = egui::Mesh::default();
    const SEGMENTS: usize = 64;
    mesh.colored_vertex(center, Color32::TRANSPARENT);
    for segment in 0..SEGMENTS {
        let angle = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let offset = Vec2::angled(angle) * radius;
        mesh.colored_vertex(
            center + offset,
            theme::with_alpha(Color32::BLACK, strength),
        );
    }
    for segment in 0..SEGMENTS as u32 {
        mesh.add_triangle(0, 1 + segment, 1 + (segment + 1) % SEGMENTS as u32);
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn paint_vertical_gradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    theme::vertical_gradient(painter, rect, top, bottom);
}

fn paint_ambient_glow(painter: &Painter, viewport: Rect, color: Color32) {
    let center = Pos2::new(
        viewport.min.x + viewport.width() * 0.64,
        viewport.min.y + viewport.height() * 0.38,
    );
    let radius = viewport.width() * 0.55;
    let glow = theme::lerp_color(color, Color32::WHITE, 0.10);

    // Un seul maillage radial : le halo était auparavant composé de 48 disques
    // superposés, ce qui saturait le remplissage du GPU (30 FPS sur UHD 600).
    const SEGMENTS: usize = 64;
    const RINGS: [f32; 4] = [0.0, 0.45, 0.75, 1.0];
    const ALPHAS: [f32; 4] = [0.10, 0.065, 0.025, 0.0];

    let mut mesh = egui::Mesh::default();
    for (ring, &fraction) in RINGS.iter().enumerate() {
        let alpha = ALPHAS[ring];
        for segment in 0..SEGMENTS {
            let angle = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            let offset = Vec2::angled(angle) * radius * fraction;
            mesh.colored_vertex(center + offset, theme::with_alpha(glow, alpha));
        }
    }

    let segments = SEGMENTS as u32;
    for ring in 0..RINGS.len() - 1 {
        let base = (ring * SEGMENTS) as u32;
        let next = base + segments;
        for segment in 0..segments {
            let a = base + segment;
            let b = base + (segment + 1) % segments;
            let c = next + segment;
            let d = next + (segment + 1) % segments;
            mesh.add_triangle(a, c, d);
            if ring > 0 {
                mesh.add_triangle(a, d, b);
            }
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn draw_horizontal_fade(painter: &Painter, rect: Rect, alpha: f32) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), theme::with_alpha(Color32::BLACK, alpha));
    mesh.colored_vertex(rect.right_top(), theme::with_alpha(Color32::BLACK, 0.0));
    mesh.colored_vertex(rect.left_bottom(), theme::with_alpha(Color32::BLACK, alpha));
    mesh.colored_vertex(
        rect.right_bottom(),
        theme::with_alpha(Color32::BLACK, 0.0),
    );
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(egui::Shape::mesh(mesh));
}
