//! `src/components/game_tile.rs` — tuile carrée individuelle du carrousel.
//!
//! Reprend les codes de la Xbox Series X : artwork carré, coins arrondis,
//! ombre portée, anneau blanc et halo sur la tuile sélectionnée.

use egui::{Color32, Painter, Pos2, Rect, Rounding, Stroke, TextureHandle, Ui, Vec2};

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
    /// Progression du reflet de sélection (0 → 1 ; 1 = terminé / inactif).
    pub shine: f32,
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

    // Artwork (couverture) ou dégradé procédural, toujours arrondi.
    let texture = visual.cover.unwrap_or(visual.gradient);
    egui::Image::new(egui::load::SizedTexture::new(texture.id(), rect.size()))
        .uv(theme::cover_uv(texture.size()))
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

    // Reflet de sélection (une passe diagonale à l'arrivée du focus).
    if style.focused {
        paint_shine(painter, rect, rounding.nw, alpha, style.shine);
    }

    // Bordure fine et discrète : le sélecteur bleu global signale la sélection.
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, alpha * 0.10)),
    );
}

/// Reflet de sélection façon PlayStation 5 : une seule bande de lumière
/// diagonale, large et très douce, qui traverse la tuile de gauche à droite
/// quand elle reçoit le focus, puis s'efface. `progress` va de 0 (focus tout
/// juste reçu) à 1 (reflet terminé).
///
/// La bande est un maillage conforme aux coins arrondis de la tuile : elle ne
/// déborde jamais, quelle que soit sa position. Les couleurs par sommet sont
/// interpolées par le GPU, donc aucun palier visible.
fn paint_shine(painter: &Painter, rect: Rect, radius: f32, alpha: f32, progress: f32) {
    /// Attente avant le départ : le ressort d'échelle a le temps de se poser.
    const DELAY: f32 = 0.10;
    /// Opacité maximale du blanc au centre de la bande.
    const PEAK: f32 = 0.20;
    const ROWS: usize = 40;
    const COLS: usize = 28;

    if progress >= 1.0 || alpha <= 0.01 {
        return;
    }
    let t = ((progress - DELAY) / (1.0 - DELAY)).clamp(0.0, 1.0);
    if t <= 0.0 {
        return;
    }
    // Départ et arrivée en douceur.
    let eased = t * t * (3.0 - 2.0 * t);

    // Direction de la lumière : de l'angle haut-gauche vers l'angle bas-droit
    // de la tuile. La bande est perpendiculaire à cette diagonale.
    let diagonal = rect.size();
    let length = diagonal.length().max(1.0);
    let direction = Vec2::new(diagonal.x / length, diagonal.y / length);
    let half_width = length * 0.26;
    // Position du bord haut-gauche (0) au bord bas-droit (length) le long de la
    // diagonale ; la bande part entièrement hors de la tuile et en ressort.
    let center = -half_width + (length + 2.0 * half_width) * eased;
    let peak = PEAK * alpha;

    let radius = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let mut mesh = egui::Mesh::default();
    for row in 0..=ROWS {
        let y = rect.min.y + rect.height() * (row as f32 / ROWS as f32);
        // Retrait horizontal dû à l'arrondi du coin à cette hauteur.
        let dy = if y < rect.min.y + radius {
            rect.min.y + radius - y
        } else if y > rect.max.y - radius {
            y - (rect.max.y - radius)
        } else {
            0.0
        };
        let inset = radius - (radius * radius - dy * dy).max(0.0).sqrt();
        let left = rect.min.x + inset;
        let right = rect.max.x - inset;
        for col in 0..=COLS {
            let x = left + (right - left) * (col as f32 / COLS as f32);
            // Projection sur la diagonale haut-gauche → bas-droit.
            let along = (x - rect.min.x) * direction.x + (y - rect.min.y) * direction.y;
            let k = (1.0 - (along - center).abs() / half_width).clamp(0.0, 1.0);
            let strength = k * k * (3.0 - 2.0 * k);
            mesh.colored_vertex(
                Pos2::new(x, y),
                theme::with_alpha(Color32::WHITE, peak * strength),
            );
        }
    }
    let stride = (COLS + 1) as u32;
    for row in 0..ROWS as u32 {
        for col in 0..COLS as u32 {
            let a = row * stride + col;
            let b = a + 1;
            let c = a + stride;
            let d = c + 1;
            mesh.add_triangle(a, b, c);
            mesh.add_triangle(b, d, c);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}
