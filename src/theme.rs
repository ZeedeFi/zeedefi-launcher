//! `src/theme.rs` — design system : palette, métriques, ombres, dégradés.
//!
//! Toutes les valeurs de rendu sont centralisées ici afin de garantir un look
//! homogène entre le fond, l'en-tête, le hero et le carrousel.

use egui::{Color32, Painter, Pos2, Rect, Rounding, Stroke, Vec2};

/// Accent principal (cyan « Big Picture »).
pub const ACCENT: Color32 = Color32::from_rgb(0x4C, 0xC2, 0xFF);
/// Accent profond (dégradé des boutons).
pub const ACCENT_DEEP: Color32 = Color32::from_rgb(0x1F, 0x6F, 0xEB);

/// Opacité de l'overlay sombre posé sur le fond d'écran.
pub const OVERLAY_ALPHA: f32 = 0.48;

/// Tuiles du carrousel : carrées.
pub const TILE_SIZE: Vec2 = Vec2::new(168.0, 168.0);
/// Écart entre deux tuiles.
pub const TILE_SPACING: f32 = 20.0;
/// Facteur d'échelle de la tuile focus (style Xbox Series).
pub const FOCUS_SCALE: f32 = 2.2;
/// Espace réservé sous la rangée de tuiles (barre d'indices).
pub const ROW_BOTTOM_PADDING: f32 = 92.0;
/// Lerp du déplacement horizontal des tuiles.
pub const SCROLL_LERP: f32 = 0.16;
/// Lerp de l'échelle animée des tuiles.
pub const SCALE_LERP: f32 = 0.20;
/// Lerp des fondus.
pub const FADE_LERP: f32 = 0.14;

/// Hauteur de l'en-tête.
pub const HEADER_HEIGHT: f32 = 96.0;
/// Marge latérale du contenu.
pub const CONTENT_MARGIN: f32 = 64.0;
/// Hauteur réservée au carrousel (tuile focus + zone d'indices).
pub const CAROUSEL_HEIGHT: f32 = TILE_SIZE.y * FOCUS_SCALE + ROW_BOTTOM_PADDING;
/// Largeur du menu de profil.
pub const PROFILE_MENU_WIDTH: f32 = 322.0;
/// Largeur du panneau de paramètres.
pub const SETTINGS_PANEL_WIDTH: f32 = 460.0;
/// Pas de navigation « page ».
pub const PAGE_STEP: usize = 5;
/// Durée d'affichage d'un toast (secondes).
pub const TOAST_DURATION: f32 = 3.2;

/// Palette de couleurs.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub surface: Color32,
    pub surface_elevated: Color32,
    pub surface_hover: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub divider: Color32,
    pub scrim: Color32,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: Color32::from_rgb(0x0D, 0x11, 0x18),
            surface: Color32::from_rgb(0x15, 0x1A, 0x24),
            surface_elevated: Color32::from_rgb(0x1B, 0x21, 0x2D),
            surface_hover: Color32::from_rgb(0x25, 0x2D, 0x3B),
            text_primary: Color32::from_rgb(0xF4, 0xF7, 0xFB),
            text_secondary: Color32::from_rgb(0xA5, 0xB0, 0xC0),
            text_muted: Color32::from_rgb(0x6C, 0x78, 0x8A),
            accent: ACCENT,
            success: Color32::from_rgb(0x4C, 0xD9, 0x73),
            warning: Color32::from_rgb(0xFF, 0xC1, 0x4C),
            danger: Color32::from_rgb(0xFF, 0x6B, 0x6B),
            divider: Color32::from_rgba_premultiplied(0x2A, 0x32, 0x40, 0x66),
            scrim: Color32::from_rgba_premultiplied(0x00, 0x00, 0x00, 0x8C),
        }
    }
}

/// Applique une opacité à une couleur.
pub fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

/// Mélange linéaire de deux couleurs (alpha compris).
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
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

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> Color32 {
    let h = hue.rem_euclid(1.0) * 6.0;
    let s = saturation.clamp(0.0, 1.0);
    let v = value.clamp(0.0, 1.0);
    let sector = h.floor();
    let f = h - sector;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match sector as i32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Color32::from_rgb(
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

/// Dégradé de repli déterministe dérivé de l'identifiant du jeu.
pub fn fallback_gradient(game_id: &str) -> [Color32; 2] {
    let hash = fnv1a(game_id.as_bytes());
    let hue = (hash % 360) as f32 / 360.0;
    let top = hsv_to_rgb(hue, 0.52, 0.54);
    let bottom = hsv_to_rgb((hue + 0.10).rem_euclid(1.0), 0.76, 0.17);
    [top, bottom]
}

/// Initiales affichées sur une tuile sans artwork.
pub fn initials(title: &str) -> String {
    let letters: String = title
        .split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .collect::<String>()
        .to_uppercase();
    if letters.is_empty() {
        "?".to_string()
    } else {
        letters
    }
}

/// Police proportionnelle standard.
pub fn font(size: f32) -> egui::FontId {
    egui::FontId::proportional(size)
}

/// Police « semi-gras » (famille personnalisée installée au démarrage).
pub fn bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("bold".into()))
}

/// Coins arrondis des tuiles au repos.
pub fn tile_radius() -> Rounding {
    Rounding::same(14.0)
}

/// Coins arrondis de la tuile focus.
pub fn focus_radius() -> Rounding {
    Rounding::same(20.0)
}

/// Coins arrondis des panneaux.
pub fn panel_radius() -> Rounding {
    Rounding::same(18.0)
}

/// Ombre douce : halo de plusieurs rectangles translucides.
///
/// À appeler *avant* de peindre le contenu, qui recouvrira le centre.
pub fn soft_shadow(painter: &Painter, rect: Rect, rounding: Rounding, strength: f32) {
    for layer in 1..=4 {
        let expand = layer as f32 * 2.5;
        let alpha = strength * 0.22 * (1.0 - layer as f32 / 5.0);
        painter.rect_filled(
            rect.expand(expand).translate(Vec2::new(0.0, layer as f32 * 0.8)),
            rounding,
            with_alpha(Color32::BLACK, alpha),
        );
    }
}

/// Halo lumineux autour de la tuile focus.
pub fn focus_glow(painter: &Painter, rect: Rect, rounding: Rounding) {
    for layer in 1..=3 {
        let expand = layer as f32 * 2.0;
        let alpha = 0.16 / layer as f32;
        painter.rect_filled(
            rect.expand(expand),
            rounding,
            with_alpha(ACCENT, alpha),
        );
    }
}

/// Remplit un rectangle avec un dégradé vertical (approximation par bandes).
pub fn vertical_gradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let steps = 24usize;
    let height = rect.height();
    if height <= 0.0 {
        return;
    }
    for step in 0..steps {
        let t0 = step as f32 / steps as f32;
        let t1 = (step + 1) as f32 / steps as f32;
        let mid = (t0 + t1) * 0.5;
        let y0 = rect.min.y + t0 * height;
        let y1 = rect.min.y + t1 * height;
        let band = Rect::from_min_max(
            Pos2::new(rect.min.x, y0),
            Pos2::new(rect.max.x, y1 + 1.0),
        );
        painter.rect_filled(band, Rounding::ZERO, lerp_color(top, bottom, mid));
    }
}

/// Dégradé vertical avec fondu d'opacité.
pub fn vertical_gradient_fade(
    painter: &Painter,
    rect: Rect,
    color: Color32,
    top_alpha: f32,
    bottom_alpha: f32,
) {
    let steps = 20usize;
    let height = rect.height();
    if height <= 0.0 {
        return;
    }
    for step in 0..steps {
        let t0 = step as f32 / steps as f32;
        let t1 = (step + 1) as f32 / steps as f32;
        let alpha = top_alpha + (bottom_alpha - top_alpha) * ((t0 + t1) * 0.5);
        let band = Rect::from_min_max(
            Pos2::new(rect.min.x, rect.min.y + t0 * height),
            Pos2::new(rect.max.x, rect.min.y + t1 * height + 1.0),
        );
        painter.rect_filled(band, Rounding::ZERO, with_alpha(color, alpha));
    }
}

/// Dessine un texte tronqué avec une ellipse s'il dépasse `max_width`.
///
/// `pos` est interprété selon `anchor` (le point d'ancrage du texte).
pub fn ellipsis_text(
    painter: &Painter,
    pos: Pos2,
    anchor: egui::Align2,
    text: &str,
    font: egui::FontId,
    color: Color32,
    max_width: f32,
) {
    let galley = painter.layout_no_wrap(text.to_string(), font.clone(), color);
    let galley = if galley.size().x <= max_width {
        galley
    } else {
        let mut truncated = String::new();
        for ch in text.chars() {
            let candidate = format!("{truncated}{ch}…");
            let width = painter
                .layout_no_wrap(candidate.clone(), font.clone(), color)
                .size()
                .x;
            if width > max_width {
                break;
            }
            truncated.push(ch);
        }
        painter.layout_no_wrap(format!("{truncated}…"), font, color)
    };
    let aligned = anchor.align_size_within_rect(
        galley.size(),
        Rect::from_min_size(pos, Vec2::ZERO),
    );
    painter.galley(aligned.min, galley, color);
}

/// Ligne séparatrice discrète.
pub fn divider(painter: &Painter, from: Pos2, to: Pos2) {
    painter.line_segment([from, to], Stroke::new(1.0_f32, Palette::default().divider));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_gradient_is_deterministic() {
        assert_eq!(
            fallback_gradient("zeedefi-game-01"),
            fallback_gradient("zeedefi-game-01")
        );
    }

    #[test]
    fn fallback_gradient_varies() {
        assert_ne!(fallback_gradient("game-a"), fallback_gradient("game-b"));
    }

    #[test]
    fn initials_handles_empty() {
        assert_eq!(initials("   "), "?");
        assert_eq!(initials("Halo Infinite"), "HI");
    }

    #[test]
    fn lerp_color_midpoint() {
        let black = Color32::from_rgb(0, 0, 0);
        let white = Color32::from_rgb(255, 255, 255);
        let mid = lerp_color(black, white, 0.5);
        assert_eq!(mid.r(), 128);
    }
}
