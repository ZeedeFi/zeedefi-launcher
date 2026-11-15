//! `src/components/profile_menu.rs` — menu déroulant du profil.
//!
//! Regroupe les actions de session : profils, paramètres, veille, extinction et
//! fermeture du launcher. Aucun processus local n'est lancé : les actions
//! système sont déléguées au démon ZeedeFi par `app.rs`.

use egui::{Color32, Id, LayerId, Order, Pos2, Rect, Rounding, Stroke, Vec2};

use crate::components::icons;
use crate::theme;

/// Actions proposées par le menu de profil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileAction {
    SwitchProfile,
    ManageUsers,
    Settings,
    Sleep,
    Shutdown,
    Quit,
}

/// Résultat du rendu du menu.
#[derive(Debug, Default, Clone, Copy)]
pub struct MenuOutput {
    pub action: Option<ProfileAction>,
    pub close: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Icon {
    Users,
    Person,
    Sliders,
    Moon,
    Power,
    Exit,
}

#[derive(Debug, Clone, Copy)]
struct Item {
    action: ProfileAction,
    label: &'static str,
    icon: Icon,
    danger: bool,
    separator_before: bool,
}

const ITEMS: [Item; 6] = [
    Item {
        action: ProfileAction::SwitchProfile,
        label: "Changer de profil",
        icon: Icon::Users,
        danger: false,
        separator_before: false,
    },
    Item {
        action: ProfileAction::ManageUsers,
        label: "Gérer les utilisateurs",
        icon: Icon::Person,
        danger: false,
        separator_before: false,
    },
    Item {
        action: ProfileAction::Settings,
        label: "Paramètres",
        icon: Icon::Sliders,
        danger: false,
        separator_before: true,
    },
    Item {
        action: ProfileAction::Sleep,
        label: "Mettre en veille",
        icon: Icon::Moon,
        danger: false,
        separator_before: true,
    },
    Item {
        action: ProfileAction::Shutdown,
        label: "Éteindre",
        icon: Icon::Power,
        danger: false,
        separator_before: false,
    },
    Item {
        action: ProfileAction::Quit,
        label: "Quitter le launcher",
        icon: Icon::Exit,
        danger: true,
        separator_before: false,
    },
];

/// Nombre d'entrées du menu.
pub fn item_count() -> usize {
    ITEMS.len()
}

/// Action associée à une entrée, si l'indice est valide.
pub fn action_at(index: usize) -> Option<ProfileAction> {
    ITEMS.get(index).map(|item| item.action)
}

const HEADER_HEIGHT: f32 = 78.0;
const ROW_HEIGHT: f32 = 46.0;
const SEPARATOR_HEIGHT: f32 = 10.0;
const PANEL_PADDING: f32 = 8.0;

/// Dessine le menu sous `anchor` (zone du bouton profil).
pub fn draw(
    context: &egui::Context,
    anchor: Rect,
    profile_name: &str,
    selected: usize,
) -> MenuOutput {
    let palette = theme::Palette::default();
    let screen = context.screen_rect();
    let width = theme::PROFILE_MENU_WIDTH;
    let separator_count = ITEMS.iter().filter(|item| item.separator_before).count();
    let height = HEADER_HEIGHT
        + PANEL_PADDING * 2.0
        + ROW_HEIGHT * ITEMS.len() as f32
        + SEPARATOR_HEIGHT * separator_count as f32;

    let max_x = (screen.max.x - width - 16.0).max(16.0);
    let x = anchor.min.x.clamp(16.0, max_x);
    let mut y = anchor.max.y + 10.0;
    if y + height > screen.max.y - 16.0 {
        y = (anchor.min.y - height - 10.0).max(16.0);
    }
    let panel_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height));

    let painter = context.layer_painter(LayerId::new(Order::Foreground, Id::new("profile_menu")));
    let rounding = Rounding::same(18.0);
    theme::soft_shadow(&painter, panel_rect, rounding, 1.0);
    painter.rect_filled(panel_rect, rounding, palette.surface_elevated);
    painter.rect_stroke(
        panel_rect,
        rounding,
        Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.10)),
    );

    // Bandeau de compte.
    let avatar_center = Pos2::new(panel_rect.min.x + 20.0 + 17.0, panel_rect.min.y + 39.0);
    painter.circle_filled(avatar_center, 19.0, theme::with_alpha(theme::ACCENT, 0.30));
    painter.circle_filled(
        avatar_center,
        17.0,
        theme::lerp_color(theme::ACCENT_DEEP, theme::ACCENT, 0.35),
    );
    icons::person(
        &painter,
        Rect::from_center_size(avatar_center, Vec2::splat(20.0)),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(panel_rect.min.x + 66.0, panel_rect.min.y + 32.0),
        egui::Align2::LEFT_CENTER,
        profile_name,
        theme::bold(15.0),
        palette.text_primary,
    );
    painter.text(
        Pos2::new(panel_rect.min.x + 66.0, panel_rect.min.y + 50.0),
        egui::Align2::LEFT_CENTER,
        "Compte local · ZeedeFi OS",
        theme::font(11.5),
        palette.text_secondary,
    );
    let header_separator_y = panel_rect.min.y + HEADER_HEIGHT;
    painter.line_segment(
        [
            Pos2::new(panel_rect.min.x + 14.0, header_separator_y),
            Pos2::new(panel_rect.max.x - 14.0, header_separator_y),
        ],
        Stroke::new(1.0_f32, palette.divider),
    );

    let pointer = context.input(|input| input.pointer.hover_pos());
    let primary_clicked = context.input(|input| input.pointer.primary_clicked());
    let mut output = MenuOutput::default();

    let mut row_y = header_separator_y + PANEL_PADDING;
    for (index, item) in ITEMS.iter().enumerate() {
        if item.separator_before {
            let separator_y = row_y + SEPARATOR_HEIGHT * 0.5;
            painter.line_segment(
                [
                    Pos2::new(panel_rect.min.x + 14.0, separator_y),
                    Pos2::new(panel_rect.max.x - 14.0, separator_y),
                ],
                Stroke::new(1.0_f32, palette.divider),
            );
            row_y += SEPARATOR_HEIGHT;
        }

        let row_rect = Rect::from_min_size(
            Pos2::new(panel_rect.min.x + 8.0, row_y),
            Vec2::new(width - 16.0, ROW_HEIGHT),
        );
        let hovered = pointer.map(|pos| row_rect.contains(pos)).unwrap_or(false);
        if hovered {
            context.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let active = hovered || index == selected;

        if active {
            let row_rounding = Rounding::same(10.0);
            painter.rect_filled(
                row_rect,
                row_rounding,
                theme::with_alpha(Color32::WHITE, if hovered { 0.11 } else { 0.07 }),
            );
            if index == selected {
                painter.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(row_rect.min.x + 4.0, row_rect.min.y + 9.0),
                        Vec2::new(3.0, ROW_HEIGHT - 18.0),
                    ),
                    Rounding::same(2.0),
                    theme::ACCENT,
                );
            }
        }

        let icon_color = if item.danger {
            palette.danger
        } else if active {
            palette.text_primary
        } else {
            palette.text_secondary
        };
        let icon_rect = Rect::from_center_size(
            Pos2::new(row_rect.min.x + 25.0, row_rect.center().y),
            Vec2::splat(20.0),
        );
        let row_background = if active {
            theme::lerp_color(palette.surface_elevated, Color32::WHITE, 0.10)
        } else {
            palette.surface_elevated
        };
        match item.icon {
            Icon::Users => icons::users(&painter, icon_rect, icon_color),
            Icon::Person => icons::person(&painter, icon_rect, icon_color),
            Icon::Sliders => icons::sliders(&painter, icon_rect, icon_color),
            Icon::Moon => icons::moon(&painter, icon_rect, icon_color, row_background),
            Icon::Power => icons::power(&painter, icon_rect, icon_color),
            Icon::Exit => icons::exit(&painter, icon_rect, icon_color),
        }

        painter.text(
            Pos2::new(row_rect.min.x + 50.0, row_rect.center().y),
            egui::Align2::LEFT_CENTER,
            item.label,
            theme::font(14.5),
            if item.danger {
                palette.danger
            } else {
                palette.text_primary
            },
        );

        if hovered && primary_clicked {
            output.action = Some(item.action);
            output.close = true;
        }

        row_y += ROW_HEIGHT;
    }

    // Clic à l'extérieur : fermeture.
    if primary_clicked && !output.close {
        if let Some(pos) = pointer {
            if !panel_rect.contains(pos) && !anchor.contains(pos) {
                output.close = true;
            }
        }
    }

    output
}
