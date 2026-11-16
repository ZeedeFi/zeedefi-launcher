//! `src/components/profile_menu.rs` — menu déroulant du profil.
//!
//! Trois groupes lisibles (Profil, Système, Launcher) avec une légende par
//! groupe. La ligne active n'est pas peinte ici : `draw` retourne son
//! rectangle et le sélecteur bleu unique (dessiné par `app.rs`) l'encadre.
//! Aucun processus local n'est lancé : les actions système sont déléguées au
//! démon ZeedeFi par `app.rs`.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Icon {
    Users,
    Person,
    Sliders,
    Moon,
    Power,
    Exit,
}

/// Regroupement visuel des entrées.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    Profile,
    System,
    Launcher,
}

const GROUPS: [Group; 3] = [Group::Profile, Group::System, Group::Launcher];

impl Group {
    fn caption(self) -> &'static str {
        match self {
            Group::Profile => "PROFIL",
            Group::System => "SYSTÈME",
            Group::Launcher => "LAUNCHER",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Item {
    action: ProfileAction,
    label: &'static str,
    icon: Icon,
    danger: bool,
    group: Group,
}

const ITEMS: [Item; 6] = [
    Item {
        action: ProfileAction::SwitchProfile,
        label: "Changer de profil",
        icon: Icon::Users,
        danger: false,
        group: Group::Profile,
    },
    Item {
        action: ProfileAction::ManageUsers,
        label: "Gérer les utilisateurs",
        icon: Icon::Person,
        danger: false,
        group: Group::Profile,
    },
    Item {
        action: ProfileAction::Settings,
        label: "Paramètres",
        icon: Icon::Sliders,
        danger: false,
        group: Group::System,
    },
    Item {
        action: ProfileAction::Sleep,
        label: "Mettre en veille",
        icon: Icon::Moon,
        danger: false,
        group: Group::System,
    },
    Item {
        action: ProfileAction::Shutdown,
        label: "Éteindre",
        icon: Icon::Power,
        danger: false,
        group: Group::System,
    },
    Item {
        action: ProfileAction::Quit,
        label: "Quitter le launcher",
        icon: Icon::Exit,
        danger: true,
        group: Group::Launcher,
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

const WIDTH: f32 = 340.0;
const HEADER_HEIGHT: f32 = 84.0;
const CAPTION_HEIGHT: f32 = 26.0;
const ROW_HEIGHT: f32 = 46.0;
const GROUP_GAP: f32 = 8.0;
const PADDING: f32 = 10.0;

/// Hauteur totale du panneau, calculée à partir du contenu.
fn panel_height() -> f32 {
    HEADER_HEIGHT
        + PADDING
        + GROUPS.len() as f32 * (CAPTION_HEIGHT + GROUP_GAP)
        + ITEMS.len() as f32 * ROW_HEIGHT
        + PADDING
}

/// Dessine le menu sous `anchor` (zone du bouton profil) et retourne le
/// rectangle de l'entrée `selected`, que le sélecteur global doit encadrer.
///
/// `reveal` va de 0.0 (menu replié, glissé vers le haut) à 1.0 (menu complet) :
/// le panneau se déroule de haut en bas à l'ouverture et se referme à la sortie.
pub fn draw(
    context: &egui::Context,
    anchor: Rect,
    profile_name: &str,
    selected: usize,
    reveal: f32,
) -> Option<Rect> {
    let palette = theme::Palette::default();
    let screen = context.screen_rect();
    let height = panel_height();

    let max_x = (screen.max.x - WIDTH - 16.0).max(16.0);
    let x = anchor.min.x.clamp(16.0, max_x);
    let mut y = anchor.max.y + 10.0;
    if y + height > screen.max.y - 16.0 {
        y = (anchor.min.y - height - 10.0).max(16.0);
    }

    let reveal = reveal.clamp(0.0, 1.0);
    let panel_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(WIDTH, height))
        .translate(Vec2::new(0.0, (1.0 - reveal) * -14.0));

    // Le contenu n'apparaît qu'au fur et à mesure du déroulement.
    let layer = context.layer_painter(LayerId::new(Order::Foreground, Id::new("profile_menu")));
    let painter = if reveal >= 0.999 {
        layer
    } else {
        let visible_bottom = panel_rect.min.y + panel_rect.height() * reveal;
        layer.with_clip_rect(Rect::from_min_max(
            Pos2::new(screen.min.x, screen.min.y),
            Pos2::new(screen.max.x, visible_bottom),
        ))
    };

    // Carte nette, sans ombre portée sombre.
    let rounding = Rounding::same(16.0);
    painter.rect_filled(panel_rect, rounding, palette.surface_elevated);
    painter.rect_stroke(
        panel_rect,
        rounding,
        Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.10)),
    );

    // --- Bandeau de compte ---
    let avatar_center = Pos2::new(panel_rect.min.x + 20.0 + 19.0, panel_rect.min.y + 40.0);
    painter.circle_filled(avatar_center, 21.0, theme::with_alpha(theme::ACCENT, 0.30));
    painter.circle_filled(
        avatar_center,
        19.0,
        theme::lerp_color(theme::ACCENT_DEEP, theme::ACCENT, 0.35),
    );
    icons::person(
        &painter,
        Rect::from_center_size(avatar_center, Vec2::splat(22.0)),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(panel_rect.min.x + 72.0, panel_rect.min.y + 32.0),
        egui::Align2::LEFT_CENTER,
        profile_name,
        theme::bold(16.0),
        palette.text_primary,
    );
    painter.text(
        Pos2::new(panel_rect.min.x + 72.0, panel_rect.min.y + 52.0),
        egui::Align2::LEFT_CENTER,
        "Compte local · ZeedeFi OS",
        theme::font(11.5),
        palette.text_secondary,
    );
    let header_bottom = panel_rect.min.y + HEADER_HEIGHT;
    painter.line_segment(
        [
            Pos2::new(panel_rect.min.x + 16.0, header_bottom),
            Pos2::new(panel_rect.max.x - 16.0, header_bottom),
        ],
        Stroke::new(1.0_f32, palette.divider),
    );

    // --- Groupes d'actions ---
    let mut selected_rect = None;
    let mut cursor_y = header_bottom + PADDING;
    for group in GROUPS {
        painter.text(
            Pos2::new(panel_rect.min.x + 20.0, cursor_y + CAPTION_HEIGHT * 0.5),
            egui::Align2::LEFT_CENTER,
            group.caption(),
            theme::bold(10.0),
            theme::with_alpha(palette.text_muted, 0.95),
        );
        cursor_y += CAPTION_HEIGHT;

        for (index, item) in ITEMS
            .iter()
            .enumerate()
            .filter(|(_, item)| item.group == group)
        {
            let row_rect = Rect::from_min_size(
                Pos2::new(panel_rect.min.x + 8.0, cursor_y),
                Vec2::new(WIDTH - 16.0, ROW_HEIGHT),
            );
            if index == selected {
                selected_rect = Some(row_rect);
            }
            paint_row(&painter, row_rect, item, index == selected);
            cursor_y += ROW_HEIGHT;
        }
        cursor_y += GROUP_GAP;
    }

    selected_rect
}

/// Dessine une entrée. La ligne active est seulement éclaircie : son
/// encadrement vient du sélecteur global.
fn paint_row(painter: &egui::Painter, rect: Rect, item: &Item, active: bool) {
    let palette = theme::Palette::default();

    let text_color = if item.danger {
        palette.danger
    } else if active {
        palette.text_primary
    } else {
        palette.text_secondary
    };

    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.min.x + 24.0, rect.center().y),
        Vec2::splat(20.0),
    );
    match item.icon {
        Icon::Users => icons::users(painter, icon_rect, text_color),
        Icon::Person => icons::person(painter, icon_rect, text_color),
        Icon::Sliders => icons::sliders(painter, icon_rect, text_color),
        Icon::Moon => icons::moon(painter, icon_rect, text_color, palette.surface_elevated),
        Icon::Power => icons::power(painter, icon_rect, text_color),
        Icon::Exit => icons::exit(painter, icon_rect, text_color),
    }

    painter.text(
        Pos2::new(rect.min.x + 50.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        item.label,
        theme::font(15.0),
        text_color,
    );
}
