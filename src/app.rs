//! `src/app.rs` — état global du launcher et boucle de rendu egui.
//!
//! Disposition « Big Picture » inspirée de la Xbox Series X :
//! - en-tête : profil à gauche, pilule d'état du démon à droite ;
//! - bloc d'information du jeu sélectionné au-dessus de la grande tuile ;
//! - rangée de tuiles carrées en bas, tuile focus agrandie et ancrée à gauche.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui::{Color32, Context, FontFamily, Id, Pos2, Rect, Rounding, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::cache::{AssetKind, AssetLoader, PathValidator, TextureCache};
use crate::components::{
    carousel, header, hero_tile, icons, profile_menu, settings_panel,
};
use crate::input_handler::{Action, Direction, InputManager};
use crate::renderer::BackgroundRenderer;
use crate::theme;
use crate::zeedefi_client::{ClientEvent, DaemonGame, Request, Response, ZeedeFiClient};

/// État de la connexion au démon ZeedeFi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Failed { code: u32 },
}

/// Entrée de la bibliothèque de jeux.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameEntry {
    pub id: String,
    pub title: String,
    pub cover_path: Option<PathBuf>,
    pub bg_path: Option<PathBuf>,
    pub store_icon: Option<PathBuf>,
}

/// Toast affiché temporairement à l'écran.
#[derive(Debug, Clone)]
struct Toast {
    message: String,
    kind: ToastKind,
    created_at: Instant,
}

#[derive(Debug, Clone, Copy)]
enum ToastKind {
    Info,
    Warn,
    Error,
}

impl Toast {
    fn info(message: String) -> Self {
        Self {
            message,
            kind: ToastKind::Info,
            created_at: Instant::now(),
        }
    }

    fn warn(message: String) -> Self {
        Self {
            message,
            kind: ToastKind::Warn,
            created_at: Instant::now(),
        }
    }
}

/// État global du launcher.
#[derive(Debug)]
pub struct AppState {
    pub focused_game_index: usize,
    pub games_library: Vec<GameEntry>,
    /// Position horizontale animée de chaque tuile.
    pub tile_x: Vec<f32>,
    /// Échelle animée de chaque tuile.
    pub tile_scales: Vec<f32>,
    /// Opacité animée de chaque tuile.
    pub tile_alpha: Vec<f32>,
    pub connection_status: ConnectionStatus,
    pub hero_action: usize,
    pub last_error_code: Option<u32>,
    pub show_fps: bool,
    pub blur_enabled: bool,
    pub offline: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            focused_game_index: 0,
            games_library: Vec::new(),
            tile_x: Vec::new(),
            tile_scales: Vec::new(),
            tile_alpha: Vec::new(),
            connection_status: ConnectionStatus::Disconnected,
            hero_action: 0,
            last_error_code: None,
            show_fps: std::env::var_os("ZEEDEFI_FPS").is_some(),
            blur_enabled: true,
            offline: false,
        }
    }
}

/// Application egui principale.
pub struct LauncherApp {
    pub state: AppState,
    input: InputManager,
    renderer: BackgroundRenderer,
    cache: TextureCache,
    loader: AssetLoader,
    client: ZeedeFiClient,
    validator: PathValidator,
    settings: settings_panel::SettingsPanel,
    detail_game: Option<usize>,
    toast: Option<Toast>,
    runtime: tokio::runtime::Handle,
    offline_cache_path: PathBuf,
    gradients: HashMap<String, egui::TextureHandle>,
    profile_name: String,
    profile_menu_open: bool,
    profile_menu_selected: usize,
    pending_close: bool,
    /// Curseur système masqué une fois pour toutes : l'interface se pilote
    /// uniquement au clavier ou à la manette.
    cursor_hidden: bool,
    /// État de la batterie : (pourcentage, branché sur secteur).
    battery: Option<(u8, bool)>,
    battery_checked: Instant,
}

impl LauncherApp {
    pub fn new(
        creation_context: &eframe::CreationContext<'_>,
        client: ZeedeFiClient,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        install_style(&creation_context.egui_ctx);
        let validator = PathValidator::from_env();
        let mut library = load_cached_library();
        if library.is_empty() {
            library = demo_library();
        }
        let state = AppState {
            games_library: library,
            connection_status: ConnectionStatus::Connecting,
            ..AppState::default()
        };

        let mut app = Self {
            state,
            input: InputManager::new(),
            renderer: BackgroundRenderer::new(),
            cache: TextureCache::new(),
            loader: AssetLoader::new(runtime.clone()),
            client,
            validator,
            settings: settings_panel::SettingsPanel::new(),
            detail_game: None,
            toast: None,
            runtime,
            offline_cache_path: offline_cache_path(),
            gradients: HashMap::new(),
            profile_name: profile_name(),
            profile_menu_open: false,
            profile_menu_selected: 0,
            pending_close: false,
            cursor_hidden: false,
            battery: read_battery_state(),
            battery_checked: Instant::now(),
        };
        app.reset_tile_layout();
        app
    }

    // ------------------------------------------------------------------
    // Entrées
    // ------------------------------------------------------------------

    fn dispatch(&mut self, action: Action) {
        if self.profile_menu_open {
            self.handle_profile_menu_input(&action);
            return;
        }
        if self.settings.open {
            if let Some(setting_action) = self.settings.handle_action(&action) {
                self.apply_setting_action(setting_action);
            }
            return;
        }
        if self.detail_game.is_some() {
            // La fiche détail est modale : A ou B la referment.
            if matches!(action, Action::Back | Action::Confirm) {
                self.detail_game = None;
            }
            return;
        }

        match action {
            Action::Navigate(direction) => match direction {
                Direction::Left => self.focus_by(-1),
                Direction::Right => self.focus_by(1),
                Direction::Up => self.cycle_hero_action(-1),
                Direction::Down => self.cycle_hero_action(1),
            },
            Action::Confirm => self.activate_hero_action(self.state.hero_action),
            Action::Back => {
                self.detail_game = None;
            }
            Action::Details => self.open_details(),
            Action::QuickSettings => self.open_settings(),
            Action::Profile => {
                self.profile_menu_open = !self.profile_menu_open;
                self.profile_menu_selected = 0;
            }
            Action::FastScroll(_) => {}
            Action::PageUp => self.focus_by(-(theme::PAGE_STEP as isize)),
            Action::PageDown => self.focus_by(theme::PAGE_STEP as isize),
            Action::GamepadConnected(name) => {
                self.toast = Some(Toast::info(format!("Manette connectée : {name}")));
            }
            Action::GamepadDisconnected => {
                self.toast = Some(Toast::warn("Manette déconnectée".to_string()));
            }
        }
    }

    fn handle_profile_menu_input(&mut self, action: &Action) {
        match action {
            Action::Navigate(Direction::Up) => {
                let count = profile_menu::item_count();
                self.profile_menu_selected = (self.profile_menu_selected + count - 1) % count;
            }
            Action::Navigate(Direction::Down) => {
                self.profile_menu_selected =
                    (self.profile_menu_selected + 1) % profile_menu::item_count();
            }
            Action::Confirm => {
                if let Some(selected) = profile_menu::action_at(self.profile_menu_selected) {
                    self.profile_menu_open = false;
                    self.execute_profile_action(selected);
                }
            }
            Action::Back | Action::Profile => {
                self.profile_menu_open = false;
            }
            _ => {}
        }
    }

    fn execute_profile_action(&mut self, action: profile_menu::ProfileAction) {
        match action {
            profile_menu::ProfileAction::SwitchProfile => {
                self.toast = Some(Toast::info(
                    "Aucun autre profil enregistré sur cet appareil".to_string(),
                ));
            }
            profile_menu::ProfileAction::ManageUsers => {
                self.toast = Some(Toast::info(
                    "Gestion des utilisateurs : un seul profil actif".to_string(),
                ));
            }
            profile_menu::ProfileAction::Settings => self.open_settings(),
            profile_menu::ProfileAction::Sleep => self.request_system(
                "system_sleep",
                "Mise en veille demandée au démon",
                "Veille impossible : démon ZeedeFi injoignable",
            ),
            profile_menu::ProfileAction::Shutdown => self.request_system(
                "system_shutdown",
                "Extinction demandée au démon",
                "Extinction impossible : démon ZeedeFi injoignable",
            ),
            profile_menu::ProfileAction::Quit => {
                self.client.shutdown();
                self.pending_close = true;
            }
        }
    }

    fn request_system(&mut self, method: &str, success: &str, failure: &str) {
        let sent = self
            .client
            .request(Request::new(method, serde_json::json!({})));
        self.toast = Some(if sent {
            Toast::info(success.to_string())
        } else {
            Toast::warn(failure.to_string())
        });
    }

    fn open_details(&mut self) {
        self.detail_game = Some(self.state.focused_game_index);
        self.profile_menu_open = false;
    }

    fn open_settings(&mut self) {
        self.settings.open_now();
        self.profile_menu_open = false;
        self.detail_game = None;
    }

    fn focus_by(&mut self, delta: isize) {
        if self.state.games_library.is_empty() {
            return;
        }
        let len = self.state.games_library.len() as isize;
        let current = self.state.focused_game_index as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        if next != self.state.focused_game_index {
            self.state.focused_game_index = next;
            self.state.hero_action = 0;
        }
    }

    fn cycle_hero_action(&mut self, delta: isize) {
        let next = (self.state.hero_action as isize + delta).rem_euclid(3) as usize;
        self.state.hero_action = next;
    }

    fn activate_hero_action(&mut self, index: usize) {
        match index {
            0 => self.launch_focused(),
            1 => self.open_details(),
            2 => self.open_settings(),
            _ => {}
        }
    }

    fn launch_focused(&mut self) {
        let Some(game) = self.state.games_library.get(self.state.focused_game_index) else {
            return;
        };
        // Sécurité : on ne lance jamais de processus localement. La demande est
        // déléguée au démon ZeedeFi, qui validera l'identifiant.
        let params = serde_json::json!({ "id": game.id });
        let sent = self.client.request(Request::new("launch_game", params));
        self.toast = Some(if sent {
            Toast::info(format!("Lancement de « {} » demandé", game.title))
        } else {
            Toast::warn("Démon ZeedeFi indisponible — lancement impossible".to_string())
        });
    }

    // ------------------------------------------------------------------
    // Démon
    // ------------------------------------------------------------------

    fn poll_client_events(&mut self) {
        let events: Vec<ClientEvent> = self.runtime.block_on(async {
            let mut collected = Vec::new();
            while let Some(event) = self.client.try_recv_event_async().await {
                collected.push(event);
            }
            collected
        });
        for event in events {
            match event {
                ClientEvent::Connected => {
                    info!("connecté au démon ZeedeFi");
                    self.state.connection_status = ConnectionStatus::Connected;
                    self.state.offline = false;
                    self.client
                        .request(Request::new("list_games", serde_json::json!({})));
                }
                ClientEvent::Disconnected => {
                    self.state.connection_status = ConnectionStatus::Disconnected;
                    self.state.offline = true;
                }
                ClientEvent::Response { method, response } => {
                    self.handle_response(&method, response);
                }
                ClientEvent::Error { code, daemon_code } => {
                    let code = daemon_code.unwrap_or(code as u32);
                    self.state.connection_status = ConnectionStatus::Failed { code };
                    self.state.last_error_code = Some(code);
                    if code == crate::zeedefi_client::ErrorCode::SocketMissing as u32
                        || code == crate::zeedefi_client::ErrorCode::ConnectionRefused as u32
                    {
                        self.state.offline = true;
                    }
                }
            }
        }
    }

    fn handle_response(&mut self, method: &str, response: Response) {
        if !response.success {
            self.state.last_error_code = Some(response.code);
            self.state.connection_status = ConnectionStatus::Failed {
                code: response.code,
            };
            return;
        }
        match method {
            "list_games" => {
                let games: Vec<DaemonGame> =
                    serde_json::from_value(response.data).unwrap_or_default();
                let entries: Vec<GameEntry> = games
                    .into_iter()
                    .filter_map(|game| self.to_game_entry(game))
                    .collect();
                if !entries.is_empty() {
                    self.state.games_library = entries;
                    self.state.focused_game_index = 0;
                    self.state.hero_action = 0;
                    self.reset_tile_layout();
                    self.persist_library();
                }
            }
            "launch_game" => {
                self.toast = Some(Toast::info("Lancement demandé".to_string()));
            }
            _ => {}
        }
    }

    fn to_game_entry(&self, game: DaemonGame) -> Option<GameEntry> {
        let cover_path = game
            .cover_path
            .as_deref()
            .and_then(|value| self.validator.validate(Path::new(value)).ok());
        let bg_path = game
            .background_path
            .as_deref()
            .and_then(|value| self.validator.validate(Path::new(value)).ok());
        Some(GameEntry {
            id: game.id,
            title: game.title.unwrap_or_else(|| "Sans titre".to_string()),
            cover_path,
            bg_path,
            store_icon: None,
        })
    }

    fn persist_library(&self) {
        let library = self.state.games_library.clone();
        let path = self.offline_cache_path.clone();
        self.runtime.spawn(async move {
            if let Some(parent) = path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            match serde_json::to_string_pretty(&library) {
                Ok(payload) => {
                    if let Err(error) = tokio::fs::write(&path, payload).await {
                        warn!(error = %error, "échec de la sauvegarde du cache local");
                    }
                }
                Err(error) => warn!(error = %error, "échec de sérialisation du cache local"),
            }
        });
    }

    // ------------------------------------------------------------------
    // Textures
    // ------------------------------------------------------------------

    fn sync_background(&mut self) {
        let bg_path = match self.state.games_library.get(self.state.focused_game_index) {
            Some(game) => game.bg_path.clone(),
            None => return,
        };

        let Some(path) = bg_path else {
            self.renderer.clear();
            return;
        };

        let kind = if self.state.blur_enabled {
            AssetKind::BlurredBackground
        } else {
            AssetKind::Background
        };
        self.loader
            .request(&mut self.cache, &self.validator, &path, kind);
        let key = format!("{}:{}", kind.key_tag(), path.display());
        if let Some(handle) = self.cache.get(&key) {
            self.renderer.set_background(handle.clone(), path);
        }
    }

    fn prefetch(&mut self) {
        let len = self.state.games_library.len();
        if len == 0 {
            return;
        }
        let focus = self.state.focused_game_index;
        let mut indices = vec![focus];
        for offset in 1..=4usize {
            if focus >= offset {
                indices.push(focus - offset);
            }
            if focus + offset < len {
                indices.push(focus + offset);
            }
        }
        for index in indices {
            let Some(game) = self.state.games_library.get(index) else {
                continue;
            };
            let cover = game.cover_path.clone();
            let bg = game.bg_path.clone();
            if let Some(path) = cover {
                self.loader
                    .request(&mut self.cache, &self.validator, &path, AssetKind::Cover);
            }
            if let Some(path) = bg {
                let kind = if self.state.blur_enabled {
                    AssetKind::BlurredBackground
                } else {
                    AssetKind::Background
                };
                self.loader
                    .request(&mut self.cache, &self.validator, &path, kind);
            }
        }
    }

    fn cover_handle(&self, index: usize) -> Option<egui::TextureHandle> {
        self.state
            .games_library
            .get(index)
            .and_then(|game| game.cover_path.as_ref())
            .and_then(|path| {
                self.cache
                    .get(&format!("cover:{}", path.display()))
                    .cloned()
            })
    }

    fn ensure_gradients(&mut self, context: &Context) {
        let ids: Vec<String> = self
            .state
            .games_library
            .iter()
            .map(|game| game.id.clone())
            .collect();
        for id in ids {
            let _ = self.gradient_texture(context, &id);
        }
    }

    fn gradient_texture(&mut self, context: &Context, game_id: &str) -> egui::TextureHandle {
        if let Some(handle) = self.gradients.get(game_id) {
            return handle.clone();
        }
        let image = gradient_image(&format!("{game_id}|tuile"));
        let handle = context.load_texture(
            format!("gradient:{game_id}"),
            image,
            egui::TextureOptions::LINEAR,
        );
        self.gradients.insert(game_id.to_string(), handle.clone());
        handle
    }

    // ------------------------------------------------------------------
    // Animation de la rangée
    // ------------------------------------------------------------------

    fn target_tile_x(&self, index: usize) -> f32 {
        let focus = self.state.focused_game_index as f32;
        let i = index as f32;
        let anchor = theme::CONTENT_MARGIN;
        let step = theme::TILE_SIZE.x + theme::TILE_SPACING;
        let focus_size = theme::TILE_SIZE.x * theme::FOCUS_SCALE;
        if i < focus {
            anchor - (focus - i) * step
        } else if (i - focus).abs() < 0.5 {
            anchor
        } else {
            anchor + focus_size + theme::TILE_SPACING + (i - focus - 1.0) * step
        }
    }

    fn reset_tile_layout(&mut self) {
        let count = self.state.games_library.len();
        let focus = self.state.focused_game_index;
        self.state.tile_x = (0..count).map(|index| self.target_tile_x(index)).collect();
        self.state.tile_scales = (0..count)
            .map(|index| {
                if index == focus {
                    theme::FOCUS_SCALE
                } else {
                    1.0
                }
            })
            .collect();
        self.state.tile_alpha = (0..count)
            .map(|index| if index < focus { 0.0 } else { 1.0 })
            .collect();
    }

    /// Anime positions, échelles et opacités. Retourne `true` si une animation
    /// est encore en cours (pour demander un repaint).
    fn animate_tiles(&mut self, delta_seconds: f32) -> bool {
        let count = self.state.games_library.len();
        if self.state.tile_x.len() != count {
            self.reset_tile_layout();
            return count > 0;
        }
        let focus = self.state.focused_game_index;
        let x_t = 1.0 - (-delta_seconds * 13.0).exp();
        let s_t = 1.0 - (-delta_seconds * 15.0).exp();
        let a_t = 1.0 - (-delta_seconds * 11.0).exp();
        let mut animating = false;

        for index in 0..count {
            let target_x = self.target_tile_x(index);
            let target_scale = if index == focus {
                theme::FOCUS_SCALE
            } else {
                1.0
            };
            let target_alpha = if index < focus { 0.0 } else { 1.0 };

            let dx = target_x - self.state.tile_x[index];
            let ds = target_scale - self.state.tile_scales[index];
            let da = target_alpha - self.state.tile_alpha[index];
            if dx.abs() > 0.25 || ds.abs() > 0.002 || da.abs() > 0.004 {
                animating = true;
            }
            self.state.tile_x[index] += dx * x_t;
            self.state.tile_scales[index] += ds * s_t;
            self.state.tile_alpha[index] += da * a_t;
        }
        animating
    }

    // ------------------------------------------------------------------
    // Paramètres
    // ------------------------------------------------------------------

    fn apply_setting_action(&mut self, action: settings_panel::SettingAction) {
        match action {
            settings_panel::SettingAction::ToggleBlur => {
                self.state.blur_enabled = !self.state.blur_enabled;
                self.settings.blur_enabled = self.state.blur_enabled;
            }
            settings_panel::SettingAction::ToggleFps => {
                self.state.show_fps = !self.state.show_fps;
                self.settings.show_fps = self.state.show_fps;
            }
            settings_panel::SettingAction::RefreshLibrary => {
                let sent = self
                    .client
                    .request(Request::new("list_games", serde_json::json!({})));
                self.toast = Some(if sent {
                    Toast::info("Rafraîchissement de la bibliothèque demandé".to_string())
                } else {
                    Toast::warn("Démon ZeedeFi indisponible".to_string())
                });
            }
            settings_panel::SettingAction::Reconnect => {
                self.state.connection_status = ConnectionStatus::Connecting;
                self.toast = Some(Toast::info(
                    "Nouvelle tentative de connexion au démon…".to_string(),
                ));
            }
            settings_panel::SettingAction::Sleep => self.request_system(
                "system_sleep",
                "Mise en veille demandée au démon",
                "Veille impossible : démon ZeedeFi injoignable",
            ),
            settings_panel::SettingAction::Shutdown => self.request_system(
                "system_shutdown",
                "Extinction demandée au démon",
                "Extinction impossible : démon ZeedeFi injoignable",
            ),
            settings_panel::SettingAction::Quit => {
                self.pending_close = true;
                self.client.shutdown();
            }
            settings_panel::SettingAction::Close => self.settings.close_now(),
        }
    }

    // ------------------------------------------------------------------
    // Surcouches
    // ------------------------------------------------------------------

    fn update_toast(&mut self) {
        if let Some(toast) = &self.toast {
            if toast.created_at.elapsed().as_secs_f32() > theme::TOAST_DURATION {
                self.toast = None;
            }
        }
    }

    fn draw_toast(&self, context: &Context) {
        let Some(toast) = &self.toast else {
            return;
        };
        let elapsed = toast.created_at.elapsed().as_secs_f32();
        let remaining = (theme::TOAST_DURATION - elapsed).max(0.0);
        let alpha = (remaining / 0.45).min(1.0);
        let palette = theme::Palette::default();
        let color = match toast.kind {
            ToastKind::Info => palette.accent,
            ToastKind::Warn => palette.warning,
            ToastKind::Error => palette.danger,
        };
        let screen = context.screen_rect();
        let painter = context.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            Id::new("toast"),
        ));
        let galley = painter.layout_no_wrap(
            toast.message.clone(),
            theme::font(13.5),
            theme::with_alpha(palette.text_primary, alpha),
        );
        let padding = Vec2::new(18.0, 11.0);
        let size = galley.size() + padding * 2.0;
        let rect = Rect::from_min_size(
            Pos2::new(
                screen.center().x - size.x * 0.5,
                screen.max.y - 132.0 - size.y,
            ),
            size,
        );
        theme::soft_shadow(&painter, rect, Rounding::same(12.0), 0.8);
        painter.rect_filled(
            rect,
            Rounding::same(12.0),
            theme::with_alpha(palette.surface_elevated, alpha * 0.96),
        );
        painter.rect_stroke(
            rect,
            Rounding::same(12.0),
            Stroke::new(1.0_f32, theme::with_alpha(color, alpha * 0.55)),
        );
        painter.circle_filled(
            Pos2::new(rect.min.x + padding.x + 4.0, rect.center().y),
            3.5,
            theme::with_alpha(color, alpha),
        );
        painter.galley(
            Pos2::new(
                rect.min.x + padding.x + 16.0,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            theme::with_alpha(palette.text_primary, alpha),
        );
    }

    fn draw_detail_overlay(&self, context: &Context) {
        let Some(index) = self.detail_game else {
            return;
        };
        let Some(game) = self.state.games_library.get(index) else {
            return;
        };
        let palette = theme::Palette::default();
        let screen = context.screen_rect();

        let backdrop = context.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            Id::new("detail_backdrop"),
        ));
        backdrop.rect_filled(screen, Rounding::ZERO, theme::with_alpha(Color32::BLACK, 0.62));

        let width = 640.0_f32.min(screen.width() - 96.0);
        let height = 330.0_f32.min(screen.height() - 140.0);
        let panel = Rect::from_center_size(screen.center(), Vec2::new(width, height));

        egui::Area::new(Id::new("detail_overlay"))
            .fixed_pos(panel.min)
            .order(egui::Order::Foreground)
            .show(context, |ui| {
                let (rect, _) = ui.allocate_exact_size(panel.size(), egui::Sense::hover());
                let painter = ui.painter_at(rect);
                theme::soft_shadow(&painter, rect, theme::panel_radius(), 1.0);
                painter.rect_filled(rect, theme::panel_radius(), palette.surface_elevated);
                painter.rect_stroke(
                    rect,
                    theme::panel_radius(),
                    Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.12)),
                );

                // Vignette.
                let thumb = Rect::from_min_size(
                    Pos2::new(rect.min.x + 24.0, rect.min.y + 24.0),
                    Vec2::splat(140.0),
                );
                painter.rect_filled(thumb, Rounding::same(16.0), palette.surface);
                let cover = self.cover_handle(index);
                let texture_id = cover
                    .as_ref()
                    .map(|handle| handle.id())
                    .or_else(|| self.gradients.get(&game.id).map(|handle| handle.id()));
                if let Some(texture_id) = texture_id {
                    egui::Image::new(egui::load::SizedTexture::new(texture_id, thumb.size()))
                        .rounding(Rounding::same(16.0))
                        .paint_at(ui, thumb);
                }
                if cover.is_none() {
                    painter.text(
                        thumb.center(),
                        egui::Align2::CENTER_CENTER,
                        theme::initials(&game.title),
                        theme::bold(44.0),
                        Color32::WHITE,
                    );
                }
                painter.rect_stroke(
                    thumb,
                    Rounding::same(16.0),
                    Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.12)),
                );

                // Texte.
                let text_x = thumb.max.x + 24.0;
                let text_width = (rect.max.x - 24.0 - text_x).max(120.0);
                theme::ellipsis_text(
                    &painter,
                    Pos2::new(text_x, rect.min.y + 58.0),
                    egui::Align2::LEFT_BOTTOM,
                    &game.title,
                    theme::bold(26.0),
                    palette.text_primary,
                    text_width,
                );
                painter.text(
                    Pos2::new(text_x, rect.min.y + 70.0),
                    egui::Align2::LEFT_TOP,
                    "Jeu installé  ·  ZeedeFi OS",
                    theme::font(12.5),
                    palette.text_secondary,
                );
                painter.line_segment(
                    [
                        Pos2::new(text_x, rect.min.y + 100.0),
                        Pos2::new(rect.max.x - 24.0, rect.min.y + 100.0),
                    ],
                    Stroke::new(1.0_f32, palette.divider),
                );

                let cover_label = path_label(&game.cover_path, "Dégradé généré");
                let bg_label = path_label(&game.bg_path, "Aucun artwork");
                let rows = [
                    ("IDENTIFIANT", game.id.clone()),
                    ("COUVERTURE", cover_label),
                    ("FOND D'ÉCRAN", bg_label),
                ];
                let mut y = rect.min.y + 116.0;
                for (label, value) in &rows {
                    painter.text(
                        Pos2::new(text_x, y),
                        egui::Align2::LEFT_TOP,
                        *label,
                        theme::bold(9.5),
                        theme::with_alpha(palette.text_muted, 0.95),
                    );
                    theme::ellipsis_text(
                        &painter,
                        Pos2::new(text_x, y + 16.0),
                        egui::Align2::LEFT_TOP,
                        value,
                        theme::font(12.0),
                        palette.text_secondary,
                        text_width,
                    );
                    y += 40.0;
                }

                painter.text(
                    Pos2::new(rect.min.x + 24.0, rect.max.y - 26.0),
                    egui::Align2::LEFT_CENTER,
                    "Le lancement est délégué au démon ZeedeFi — aucune exécution locale.",
                    theme::font(11.0),
                    palette.text_muted,
                );
                painter.text(
                    Pos2::new(rect.max.x - 24.0, rect.max.y - 26.0),
                    egui::Align2::RIGHT_CENTER,
                    "A / B  Fermer",
                    theme::font(11.0),
                    palette.text_muted,
                );

                // Repère de fermeture : A / B / Échap referment la fiche.
                let close_rect = Rect::from_min_size(
                    Pos2::new(rect.max.x - 54.0, rect.min.y + 18.0),
                    Vec2::splat(34.0),
                );
                icons::close(&painter, close_rect.shrink(9.0), palette.text_secondary);
            });
    }

    fn draw_hints(&self, context: &Context) {
        if self.settings.open || self.profile_menu_open || self.detail_game.is_some() {
            return;
        }
        let screen = context.screen_rect();
        let palette = theme::Palette::default();
        let painter = context.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            Id::new("hints"),
        ));
        let hints = [
            ("A", "Jouer"),
            ("X", "Détails"),
            ("Y", "Paramètres"),
            ("B", "Retour"),
            ("P", "Profil"),
        ];
        let cap = 20.0;
        let group_gap = 22.0;
        let label_font = theme::font(12.0);
        let widths: Vec<f32> = hints
            .iter()
            .map(|(_, label)| {
                cap + 7.0
                    + painter
                        .layout_no_wrap(label.to_string(), label_font.clone(), palette.text_secondary)
                        .size()
                        .x
            })
            .collect();
        let total: f32 = widths.iter().sum::<f32>() + group_gap * (hints.len() - 1) as f32;
        let y = screen.max.y - 36.0;
        let mut x = screen.max.x - theme::CONTENT_MARGIN - total;

        for ((key, label), width) in hints.iter().zip(widths.iter()) {
            let cap_rect = Rect::from_center_size(Pos2::new(x + cap * 0.5, y), Vec2::splat(cap));
            painter.rect_filled(
                cap_rect,
                Rounding::same(6.0),
                theme::with_alpha(Color32::WHITE, 0.13),
            );
            painter.rect_stroke(
                cap_rect,
                Rounding::same(6.0),
                Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.16)),
            );
            painter.text(
                cap_rect.center(),
                egui::Align2::CENTER_CENTER,
                *key,
                theme::bold(10.0),
                palette.text_primary,
            );
            painter.text(
                Pos2::new(x + cap + 7.0, y),
                egui::Align2::LEFT_CENTER,
                *label,
                label_font.clone(),
                palette.text_secondary,
            );
            x += width + group_gap;
        }
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, context: &Context, _frame: &mut eframe::Frame) {
        let screen = context.screen_rect();
        let delta = context.input(|input| input.stable_dt).clamp(0.001, 0.1);

        // L'interface est exclusivement clavier/manette : le pointeur système
        // est masqué en permanence.
        if !self.cursor_hidden {
            context.send_viewport_cmd(egui::ViewportCommand::CursorVisible(false));
            self.cursor_hidden = true;
        }

        // --- Entrées ---
        for action in self.input.poll(context) {
            self.dispatch(action);
        }

        // --- Démon ---
        self.poll_client_events();

        // --- Batterie (rafraîchie toutes les 30 s) ---
        if self.battery_checked.elapsed() >= Duration::from_secs(30) {
            self.battery = read_battery_state();
            self.battery_checked = Instant::now();
        }

        // --- Textures ---
        self.loader.poll(context, &mut self.cache);
        self.sync_background();
        self.prefetch();
        self.ensure_gradients(context);

        // --- Animations ---
        let tiles_animating = self.animate_tiles(delta);
        self.renderer.update_animation(delta);

        // --- Synchronisation du panneau de paramètres ---
        self.settings.blur_enabled = self.state.blur_enabled;
        self.settings.show_fps = self.state.show_fps;
        self.settings.connection = self.state.connection_status.clone();
        self.settings.game_count = self.state.games_library.len();
        self.settings.error_code = self.state.last_error_code;

        // --- Fond ---
        let fallback_id = self
            .state
            .games_library
            .get(self.state.focused_game_index)
            .map(|game| game.id.as_str());
        self.renderer.draw(context, screen, fallback_id);

        // --- Instantanés de textures pour les tuiles ---
        let covers: Vec<Option<egui::TextureHandle>> = (0..self.state.games_library.len())
            .map(|index| self.cover_handle(index))
            .collect();
        let fallback_gradient = self.gradient_texture(context, "__fallback__");
        let gradients: Vec<egui::TextureHandle> = self
            .state
            .games_library
            .iter()
            .map(|game| {
                self.gradients
                    .get(&game.id)
                    .cloned()
                    .unwrap_or_else(|| fallback_gradient.clone())
            })
            .collect();
        let visuals = carousel::TileVisuals {
            covers: &covers,
            gradients: &gradients,
        };

        // --- Interface principale ---
        let profile_anchor = egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(context, |ui| {
                let profile_anchor = header::draw(
                    ui,
                    &self.state,
                    &self.profile_name,
                    self.profile_menu_open,
                    self.battery,
                );

                let hero_area = Rect::from_min_size(
                    Pos2::new(0.0, theme::HEADER_HEIGHT),
                    Vec2::new(
                        screen.width(),
                        (screen.height() - theme::HEADER_HEIGHT - theme::CAROUSEL_HEIGHT)
                            .max(1.0),
                    ),
                );
                ui.allocate_ui_at_rect(hero_area, |ui| {
                    if let Some(entry) =
                        self.state.games_library.get(self.state.focused_game_index)
                    {
                        hero_tile::draw(ui, hero_area, entry, self.state.hero_action);
                    }
                });

                let carousel_area = Rect::from_min_size(
                    Pos2::new(0.0, screen.height() - theme::CAROUSEL_HEIGHT),
                    Vec2::new(screen.width(), theme::CAROUSEL_HEIGHT),
                );
                ui.allocate_ui_at_rect(carousel_area, |ui| {
                    carousel::draw(ui, carousel_area, &self.state, &visuals);
                });

                profile_anchor
            })
            .inner;

        // --- Menu de profil ---
        if self.profile_menu_open {
            profile_menu::draw(
                context,
                profile_anchor,
                &self.profile_name,
                self.profile_menu_selected,
            );
        }

        // --- Panneau de paramètres ---
        if let Some(action) = self.settings.draw(context, screen, delta) {
            self.apply_setting_action(action);
        }

        // --- Surcouche détails ---
        self.draw_detail_overlay(context);

        // --- Toast, indices, FPS ---
        self.update_toast();
        self.draw_toast(context);
        self.draw_hints(context);

        if self.state.show_fps {
            let fps = 1.0 / delta.max(0.001);
            let painter = context.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                Id::new("fps"),
            ));
            painter.text(
                Pos2::new(screen.min.x + theme::CONTENT_MARGIN, screen.max.y - 36.0),
                egui::Align2::LEFT_CENTER,
                format!("{fps:.0} FPS"),
                theme::font(11.0),
                theme::Palette::default().text_secondary,
            );
        }

        // --- Fermeture demandée ---
        if self.pending_close {
            context.send_viewport_cmd(egui::ViewportCommand::Close);
            self.pending_close = false;
        }

        // --- Repaint si nécessaire ---
        if tiles_animating
            || self.settings.is_animating()
            || self.renderer.fade < 1.0
            || self.toast.is_some()
        {
            context.request_repaint();
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::Palette::default()
            .background
            .to_normalized_gamma_f32()
    }

    fn on_exit(&mut self) {
        self.client.shutdown();
    }
}

// ----------------------------------------------------------------------
// Utilitaires
// ----------------------------------------------------------------------

fn path_label(path: &Option<PathBuf>, fallback: &str) -> String {
    path.as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| fallback.to_string())
}

fn gradient_image(game_id: &str) -> egui::ColorImage {
    let palette = theme::fallback_gradient(game_id);
    let size = 256usize;
    // Graine déterministe : le bruit casse les anneaux de quantification 8 bits
    // visibles sur les grands dégradés (tuile focus notamment).
    let mut seed = 0x9E37_79B9u32;
    for byte in game_id.bytes() {
        seed = seed
            .wrapping_mul(1_664_525)
            .wrapping_add(u32::from(byte) + 1_013_904_223);
    }
    let mut pixels = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            let t = y as f32 / (size - 1) as f32;
            let base = theme::lerp_color(palette[0], palette[1], t);
            // Éclat lumineux en haut à gauche.
            let dx = (x as f32 / (size - 1) as f32 - 0.30) / 0.85;
            let dy = (y as f32 / (size - 1) as f32 - 0.22) / 0.70;
            let distance = (dx * dx + dy * dy).sqrt().min(1.0);
            let highlight = (1.0 - distance).powf(2.2) * 0.24;
            // Vignettage doux en bas.
            let vignette = (t - 0.55).max(0.0) * 0.20;
            let color = theme::lerp_color(base, Color32::WHITE, highlight);
            let color = theme::lerp_color(color, Color32::BLACK, vignette);

            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let noise = ((seed >> 8) % 5) as i32 - 2;
            let channel = |value: u8| (i32::from(value) + noise).clamp(0, 255) as u8;
            pixels.push(egui::Color32::from_rgba_unmultiplied(
                channel(color.r()),
                channel(color.g()),
                channel(color.b()),
                color.a(),
            ));
        }
    }
    egui::ColorImage {
        size: [size, size],
        pixels,
    }
}

fn install_style(context: &Context) {
    install_fonts(context);
    let palette = theme::Palette::default();
    let mut style = (*context.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.override_text_color = Some(palette.text_primary);
    style.visuals.window_fill = palette.surface_elevated;
    style.visuals.window_rounding = theme::panel_radius();
    style.visuals.window_stroke =
        Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.10));
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.animation_time = 0.16;
    context.set_style(style);
}

fn install_fonts(context: &Context) {
    let mut fonts = egui::FontDefinitions::default();

    let regular = read_font(&[
        "fonts/Inter-Regular.ttf",
        "/usr/share/fonts/TTF/OpenSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]);
    let bold = read_font(&[
        "fonts/Inter-Bold.ttf",
        "/usr/share/fonts/TTF/OpenSans-SemiBold.ttf",
        "/usr/share/fonts/TTF/OpenSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    ]);

    let mut has_regular = false;
    if let Some(bytes) = regular {
        fonts
            .font_data
            .insert("ui".to_owned(), egui::FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "ui".to_owned());
        has_regular = true;
    }
    if let Some(bytes) = bold {
        fonts
            .font_data
            .insert("ui-bold".to_owned(), egui::FontData::from_owned(bytes));
        let mut family = vec!["ui-bold".to_owned()];
        if has_regular {
            family.push("ui".to_owned());
        }
        family.push("Ubuntu-Light".to_owned());
        fonts
            .families
            .insert(FontFamily::Name("bold".into()), family);
    }
    context.set_fonts(fonts);
}

fn read_font(candidates: &[&str]) -> Option<Vec<u8>> {
    for candidate in candidates {
        if let Some(bytes) = read_asset_bytes(candidate) {
            return Some(bytes);
        }
        if let Ok(bytes) = std::fs::read(candidate) {
            return Some(bytes);
        }
    }
    None
}

fn read_asset_bytes(relative: &str) -> Option<Vec<u8>> {
    let candidates = [
        std::env::current_dir()
            .ok()
            .map(|dir| dir.join("assets").join(relative)),
        std::env::var_os("CARGO_MANIFEST_DIR")
            .map(|dir| PathBuf::from(dir).join("assets").join(relative)),
    ];
    for candidate in candidates.into_iter().flatten() {
        if let Ok(bytes) = std::fs::read(&candidate) {
            return Some(bytes);
        }
    }
    None
}

fn offline_cache_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".cache/zeedefi-launcher/library.json")
}

fn load_cached_library() -> Vec<GameEntry> {
    let path = offline_cache_path();
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn profile_name() -> String {
    if let Ok(name) = std::env::var("ZEEDEFI_PROFILE") {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let user = std::env::var("USER").unwrap_or_default();
    if !user.is_empty() {
        // Le champ GECOS de /etc/passwd contient souvent le nom complet.
        if let Ok(passwd) = std::fs::read_to_string("/etc/passwd") {
            for line in passwd.lines() {
                let mut fields = line.split(':');
                if fields.next() != Some(user.as_str()) {
                    continue;
                }
                if let Some(gecos) = fields.nth(3) {
                    let full_name = gecos.split(',').next().unwrap_or("").trim();
                    if !full_name.is_empty() {
                        return full_name.to_string();
                    }
                }
                break;
            }
        }
        let mut chars = user.chars();
        if let Some(first) = chars.next() {
            return format!("{}{}", first.to_uppercase(), chars.as_str());
        }
    }
    "Invité".to_string()
}

fn demo_library() -> Vec<GameEntry> {
    let titles = [
        "Starfield",
        "Halo Infinite",
        "Forza Horizon",
        "Sea of Thieves",
        "Ori and the Blind Forest",
        "Grounded",
        "Age of Empires IV",
        "Flight Simulator",
    ];
    titles
        .iter()
        .enumerate()
        .map(|(index, title)| GameEntry {
            id: format!("demo-{index}"),
            title: (*title).to_string(),
            cover_path: None,
            bg_path: None,
            store_icon: None,
        })
        .collect()
}

/// Lit l'état de la première batterie présente.
fn read_battery_state() -> Option<(u8, bool)> {
    let entries = std::fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("BAT") {
            continue;
        }
        let capacity = std::fs::read_to_string(path.join("capacity"))
            .ok()?
            .trim()
            .parse::<u8>()
            .ok()?;
        let status = std::fs::read_to_string(path.join("status")).unwrap_or_default();
        let plugged = !status.trim().eq_ignore_ascii_case("Discharging");
        return Some((capacity, plugged));
    }
    None
}
