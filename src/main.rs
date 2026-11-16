#![allow(dead_code)]

//! `src/main.rs` — point d'entrée : initialisation du tracing, du runtime
//! Tokio et de la fenêtre plein écran egui/wgpu.

mod app;
mod cache;
mod components;
mod input_handler;
mod renderer;
mod theme;
mod zeedefi_client;

use std::error::Error;
use std::path::PathBuf;

use tracing_subscriber::EnvFilter;

use app::LauncherApp;
use zeedefi_client::ZeedeFiClient;

fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let socket_path = std::env::var_os(zeedefi_client::ENV_SOCKET_PATH)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(zeedefi_client::DEFAULT_SOCKET_PATH));

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("zeedefi-io")
        .enable_all()
        .build()?;

    let client = ZeedeFiClient::spawn(runtime.handle().clone(), socket_path);

    // Synchronisation verticale activée : le FPS suit le rafraîchissement de la
    // dalle (désactivable avec ZEEDEFI_NO_VSYNC pour les diagnostics).
    let mut options = eframe::NativeOptions {
        vsync: true,
        viewport: egui::ViewportBuilder::default()
            .with_title("ZeedeFi Launcher")
            .with_fullscreen(true)
            .with_app_id("org.zeedefi.launcher"),
        ..eframe::NativeOptions::default()
    };
    if std::env::var_os("ZEEDEFI_NO_VSYNC").is_some() {
        options.vsync = false;
    }

    eframe::run_native(
        "zeedefi-launcher",
        options,
        Box::new(move |creation_context| {
            let app = LauncherApp::new(creation_context, client, runtime.handle().clone());
            Box::new(app) as Box<dyn eframe::App>
        }),
    )?;

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,zeedefi_launcher=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}
