//! `src/components/icons.rs` — icônes vectorielles peintes au `Painter`.
//!
//! Évite toute dépendance à des fichiers SVG/PNG et garantit un rendu
//! identique quelle que soit la disponibilité des polices d'icônes.

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

fn center(rect: Rect) -> Pos2 {
    rect.center()
}

fn unit(rect: Rect) -> f32 {
    rect.width().min(rect.height())
}

/// Silhouette de personne (profil).
pub fn person(painter: &Painter, rect: Rect, color: Color32) {
    let u = unit(rect);
    let c = center(rect);
    let head_radius = u * 0.18;
    let head_center = Pos2::new(c.x, c.y - u * 0.16);
    painter.circle_filled(head_center, head_radius, color);
    let body = Rect::from_center_size(
        Pos2::new(c.x, c.y + u * 0.22),
        Vec2::new(u * 0.52, u * 0.30),
    );
    painter.rect_filled(body, egui::Rounding::same(u * 0.15), color);
}

/// Deux personnes (gestion des utilisateurs).
pub fn users(painter: &Painter, rect: Rect, color: Color32) {
    let u = unit(rect);
    let c = center(rect);
    for (dx, scale) in [(-u * 0.14, 0.9), (u * 0.16, 0.72)] {
        let head = Pos2::new(c.x + dx, c.y - u * 0.12);
        painter.circle_filled(head, u * 0.15 * scale, color);
        let body = Rect::from_center_size(
            Pos2::new(c.x + dx, c.y + u * 0.20),
            Vec2::new(u * 0.36 * scale, u * 0.24),
        );
        painter.rect_filled(body, egui::Rounding::same(u * 0.10), color);
    }
}

/// Curseurs de réglages.
pub fn sliders(painter: &Painter, rect: Rect, color: Color32) {
    let u = unit(rect);
    let c = center(rect);
    let stroke = Stroke::new((u * 0.06).max(1.4), color);
    for row in 0..3 {
        let y = c.y + (row as f32 - 1.0) * u * 0.20;
        let x0 = c.x - u * 0.26;
        let x1 = c.x + u * 0.26;
        painter.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], stroke);
        let knob_x = match row {
            0 => x1 - u * 0.10,
            1 => x0 + u * 0.12,
            _ => c.x + u * 0.04,
        };
        painter.circle_filled(Pos2::new(knob_x, y), u * 0.075, color);
    }
}

/// Croissant de lune (mise en veille).
pub fn moon(painter: &Painter, rect: Rect, color: Color32, background: Color32) {
    let u = unit(rect);
    let c = center(rect);
    painter.circle_filled(c, u * 0.26, color);
    // Le disque décalé « creuse » le croissant avec la couleur du panneau.
    painter.circle_filled(
        Pos2::new(c.x + u * 0.11, c.y - u * 0.09),
        u * 0.22,
        background,
    );
}

/// Symbole d'alimentation (arrêt).
pub fn power(painter: &Painter, rect: Rect, color: Color32) {
    let u = unit(rect);
    let c = center(rect);
    let radius = u * 0.24;
    let stroke = Stroke::new((u * 0.065).max(1.6), color);
    let points: Vec<Pos2> = (0..22)
        .map(|index| {
            let t = index as f32 / 21.0;
            // Arc de 300° laissant une ouverture en haut.
            let angle = -std::f32::consts::FRAC_PI_2 + 0.52 + t * (std::f32::consts::TAU - 1.04);
            Pos2::new(c.x + radius * angle.cos(), c.y + radius * angle.sin())
        })
        .collect();
    painter.add(egui::Shape::line(points, stroke));
    let stroke_top = Stroke::new((u * 0.075).max(1.8), color);
    painter.line_segment(
        [Pos2::new(c.x, c.y - radius - u * 0.10), Pos2::new(c.x, c.y - u * 0.04)],
        stroke_top,
    );
}

/// Porte + flèche (quitter).
pub fn exit(painter: &Painter, rect: Rect, color: Color32) {
    let u = unit(rect);
    let c = center(rect);
    let stroke = Stroke::new((u * 0.065).max(1.5), color);
    let door = Rect::from_center_size(
        Pos2::new(c.x - u * 0.08, c.y),
        Vec2::new(u * 0.26, u * 0.42),
    );
    painter.rect_stroke(door, egui::Rounding::same(u * 0.05), stroke);
    painter.line_segment(
        [
            Pos2::new(c.x - u * 0.04, c.y),
            Pos2::new(c.x + u * 0.27, c.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(c.x + u * 0.17, c.y - u * 0.10),
            Pos2::new(c.x + u * 0.28, c.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(c.x + u * 0.17, c.y + u * 0.10),
            Pos2::new(c.x + u * 0.28, c.y),
        ],
        stroke,
    );
}

/// Petite icône de validation (play).
pub fn play(painter: &Painter, rect: Rect, color: Color32) {
    let u = unit(rect);
    let c = center(rect);
    let points = vec![
        Pos2::new(c.x - u * 0.16, c.y - u * 0.22),
        Pos2::new(c.x + u * 0.24, c.y),
        Pos2::new(c.x - u * 0.16, c.y + u * 0.22),
    ];
    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        Stroke::NONE,
    ));
}

/// Petit « i » dans un cercle (détails).
pub fn info(painter: &Painter, rect: Rect, color: Color32) {
    let u = unit(rect);
    let c = center(rect);
    let stroke = Stroke::new((u * 0.06).max(1.4), color);
    painter.circle_stroke(c, u * 0.28, stroke);
    painter.circle_filled(Pos2::new(c.x, c.y - u * 0.12), u * 0.035, color);
    painter.line_segment(
        [Pos2::new(c.x, c.y - u * 0.03), Pos2::new(c.x, c.y + u * 0.15)],
        stroke,
    );
}

/// Chevron de menu (vers le bas, ou vers le haut si `up`).
pub fn chevron(painter: &Painter, point: Pos2, size: f32, color: Color32, up: bool) {
    let stroke = Stroke::new(1.8_f32, color);
    let (dy0, dy1) = if up { (size * 0.35, -size * 0.35) } else { (-size * 0.35, size * 0.35) };
    painter.line_segment(
        [
            Pos2::new(point.x - size * 0.5, point.y + dy0),
            Pos2::new(point.x, point.y + dy1),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(point.x, point.y + dy1),
            Pos2::new(point.x + size * 0.5, point.y + dy0),
        ],
        stroke,
    );
}

/// Croix de fermeture.
pub fn close(painter: &Painter, rect: Rect, color: Color32) {
    let u = unit(rect);
    let c = center(rect);
    let stroke = Stroke::new((u * 0.075).max(1.5), color);
    let d = u * 0.18;
    painter.line_segment(
        [Pos2::new(c.x - d, c.y - d), Pos2::new(c.x + d, c.y + d)],
        stroke,
    );
    painter.line_segment(
        [Pos2::new(c.x + d, c.y - d), Pos2::new(c.x - d, c.y + d)],
        stroke,
    );
}

/// Flèche vers la droite (action, navigation).
pub fn arrow_right(painter: &Painter, rect: Rect, color: Color32) {
    let u = unit(rect);
    let c = center(rect);
    let stroke = Stroke::new((u * 0.065).max(1.4), color);
    painter.line_segment(
        [Pos2::new(c.x - u * 0.18, c.y), Pos2::new(c.x + u * 0.18, c.y)],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(c.x + u * 0.05, c.y - u * 0.13),
            Pos2::new(c.x + u * 0.19, c.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(c.x + u * 0.05, c.y + u * 0.13),
            Pos2::new(c.x + u * 0.19, c.y),
        ],
        stroke,
    );
}
