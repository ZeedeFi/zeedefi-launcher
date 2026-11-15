//! `src/components/header.rs` — barre supérieure : profil et état du démon.
//!
//! Le profil (avatar + nom) est à gauche ; l'état de la connexion est regroupé
//! dans une pilule à droite. L'en-tête est purement informatif : le menu de
//! profil s'ouvre avec Start / Tab / P, jamais à la souris.

use egui::{Color32, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use crate::app::{AppState, ConnectionStatus};
use crate::components::icons;
use crate::theme;

/// Dessine l'en-tête et retourne la zone du bloc profil (ancre du menu).
pub fn draw(
    ui: &mut Ui,
    state: &AppState,
    profile_name: &str,
    menu_open: bool,
    battery: Option<(u8, bool)>,
) -> Rect {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(width, theme::HEADER_HEIGHT),
        Sense::hover(),
    );

    let painter = ui.painter();

    // Voile dégradé pour garantir la lisibilité sur l'artwork.
    let scrim = Rect::from_min_size(
        Pos2::new(rect.min.x, rect.min.y),
        Vec2::new(rect.width(), 190.0),
    );
    theme::vertical_gradient_fade(painter, scrim, Color32::BLACK, 0.44, 0.0);

    let palette = theme::Palette::default();
    let center_y = rect.center().y;

    // --- Profil (gauche) ---
    let avatar_radius = 22.0;
    let avatar_center = Pos2::new(rect.min.x + theme::CONTENT_MARGIN + avatar_radius, center_y);
    let name = profile_name;
    let subtitle = "Profil principal";
    let text_x = avatar_center.x + avatar_radius + 14.0;
    let name_width = painter
        .layout_no_wrap(name.to_string(), theme::bold(17.0), palette.text_primary)
        .size()
        .x;
    let subtitle_width = painter
        .layout_no_wrap(
            subtitle.to_string(),
            theme::font(12.5),
            palette.text_secondary,
        )
        .size()
        .x;
    let chevron_point = Pos2::new(
        text_x + name_width.max(subtitle_width) + 18.0,
        center_y,
    );
    let profile_rect = Rect::from_min_max(
        Pos2::new(rect.min.x + theme::CONTENT_MARGIN - 10.0, center_y - 30.0),
        Pos2::new(chevron_point.x + 14.0, center_y + 30.0),
    );

    // Fond du bloc uniquement lorsque le menu est ouvert (repère d'ancrage).
    if menu_open {
        painter.rect_filled(
            profile_rect,
            Rounding::same(14.0),
            theme::with_alpha(Color32::WHITE, 0.10),
        );
        painter.rect_stroke(
            profile_rect,
            Rounding::same(14.0),
            Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.08)),
        );
    }

    // Avatar stylisé (dégradé de disques + pictogramme).
    painter.circle_filled(
        avatar_center,
        avatar_radius + 1.0,
        theme::with_alpha(theme::ACCENT, 0.28),
    );
    painter.circle_filled(
        avatar_center,
        avatar_radius - 1.0,
        theme::lerp_color(theme::ACCENT_DEEP, theme::ACCENT, 0.35),
    );
    icons::person(
        painter,
        Rect::from_center_size(avatar_center, Vec2::splat(avatar_radius * 1.15)),
        theme::with_alpha(Color32::WHITE, 0.96),
    );
    painter.circle_stroke(
        avatar_center,
        avatar_radius + 2.0,
        Stroke::new(
            2.0_f32,
            if menu_open {
                theme::with_alpha(Color32::WHITE, 0.92)
            } else {
                theme::with_alpha(theme::ACCENT, 0.75)
            },
        ),
    );

    painter.text(
        Pos2::new(text_x, center_y - 9.0),
        egui::Align2::LEFT_CENTER,
        name,
        theme::bold(17.0),
        palette.text_primary,
    );
    painter.text(
        Pos2::new(text_x, center_y + 11.0),
        egui::Align2::LEFT_CENTER,
        subtitle,
        theme::font(12.5),
        palette.text_secondary,
    );
    icons::chevron(
        painter,
        chevron_point,
        10.0,
        theme::with_alpha(palette.text_secondary, 0.95),
        menu_open,
    );

    // --- Pilule d'état de la connexion (droite) ---
    let (label, dot_color) = match &state.connection_status {
        ConnectionStatus::Connected => ("Démon connecté".to_string(), palette.success),
        ConnectionStatus::Connecting => ("Connexion…".to_string(), palette.warning),
        ConnectionStatus::Disconnected => ("Démon hors ligne".to_string(), palette.text_muted),
        ConnectionStatus::Failed { code } => {
            // Les codes de transport (socket absent, connexion refusée) signifient
            // simplement que le démon n'est pas joignable.
            if *code == crate::zeedefi_client::ErrorCode::SocketMissing as u32
                || *code == crate::zeedefi_client::ErrorCode::ConnectionRefused as u32
            {
                ("Démon hors ligne".to_string(), palette.text_muted)
            } else {
                (format!("Erreur #{code}"), palette.danger)
            }
        }
    };
    let text_width = painter
        .layout_no_wrap(label.clone(), theme::bold(13.0), palette.text_primary)
        .size()
        .x;
    let pill_height = 36.0;
    let pill_width = 16.0 + 8.0 + 9.0 + text_width + 16.0;
    let pill_rect = Rect::from_min_size(
        Pos2::new(
            rect.max.x - theme::CONTENT_MARGIN - pill_width,
            center_y - pill_height * 0.5,
        ),
        Vec2::new(pill_width, pill_height),
    );
    let pill_rounding = Rounding::same(pill_height * 0.5);
    painter.rect_filled(
        pill_rect,
        pill_rounding,
        theme::with_alpha(Color32::BLACK, 0.40),
    );
    painter.rect_stroke(
        pill_rect,
        pill_rounding,
        Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.12)),
    );

    // --- Batterie (à gauche de la pilule d'état) ---
    if let Some((capacity, plugged)) = battery {
        let color = if plugged {
            palette.success
        } else if capacity <= 12 {
            palette.danger
        } else if capacity <= 20 {
            palette.warning
        } else {
            palette.text_secondary
        };
        let value = format!("{capacity} %");
        let value_width = painter
            .layout_no_wrap(value.clone(), theme::bold(12.0), color)
            .size()
            .x;
        let glyph = Vec2::new(24.0, 12.0);
        let total = glyph.x + 8.0 + value_width;
        let x = pill_rect.min.x - 20.0 - total;

        let body = Rect::from_min_size(
            Pos2::new(x, center_y - glyph.y * 0.5),
            glyph,
        );
        let stroke = theme::with_alpha(color, 0.55);
        painter.rect_stroke(
            body,
            Rounding::same(3.0),
            Stroke::new(1.0_f32, stroke),
        );
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(body.max.x + 1.5, center_y - 2.5),
                Vec2::new(2.0, 5.0),
            ),
            Rounding::same(1.0),
            stroke,
        );
        let inner = body.shrink(2.5);
        let fill_width = inner.width() * f32::from(capacity) / 100.0;
        if fill_width > 0.5 {
            painter.rect_filled(
                Rect::from_min_size(inner.min, Vec2::new(fill_width, inner.height())),
                Rounding::same(1.5),
                theme::with_alpha(color, 0.9),
            );
        }
        painter.text(
            Pos2::new(body.max.x + 8.0, center_y),
            egui::Align2::LEFT_CENTER,
            value,
            theme::bold(12.0),
            color,
        );
    }

    let dot_center = Pos2::new(pill_rect.min.x + 16.0 + 4.0, center_y);
    if matches!(
        state.connection_status,
        ConnectionStatus::Connected | ConnectionStatus::Connecting
    ) {
        painter.circle_filled(dot_center, 7.0, theme::with_alpha(dot_color, 0.20));
        painter.circle_filled(dot_center, 5.0, theme::with_alpha(dot_color, 0.35));
    }
    painter.circle_filled(dot_center, 3.5, dot_color);
    painter.text(
        Pos2::new(dot_center.x + 9.0, center_y),
        egui::Align2::LEFT_CENTER,
        label,
        theme::bold(13.0),
        palette.text_primary,
    );

    profile_rect
}
