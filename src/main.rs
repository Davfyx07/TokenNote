use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use gtk4::prelude::*;
use gtk4::glib;
use gtk4::Application;
use tokio::sync::Notify;

mod config;
mod data;
mod tray;
mod ui;
mod window;

use config::WidgetConfig;
use data::AiProviderUsage;
use ui::widget_view::MainWidgetView;

const APP_ID: &str = "com.davfy.aiusagewidget";

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Iniciando AI Usage Widget...");

    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    // Si ya está corriendo (p. ej. oculto en la bandeja), solo mostrarlo
    if let Some(existing) = app.windows().first() {
        existing.present();
        return;
    }

    let config = WidgetConfig::load();
    let window = window::create_overlay_window(app, &config);

    let view = Rc::new(RefCell::new(MainWidgetView::new()));
    window.set_child(Some(&view.borrow().root_container));

    // Canal seguro entre Tokio y GTK4
    // glib::MainContext::channel ya no existe en glib 0.20; el mpsc de tokio funciona en cualquier executor
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Vec<AiProviderUsage>>();

    // Icono de bandeja: mostrar/ocultar y salir; su aro indica el % restante más bajo
    let (tray_tx, mut tray_rx) = tokio::sync::mpsc::unbounded_channel::<tray::TrayCmd>();
    let tray_handle = tray::spawn(tray_tx);

    let view_clone = view.clone();
    glib::spawn_future_local(async move {
        while let Some(providers) = receiver.recv().await {
            if let Some(handle) = &tray_handle {
                tray::set_level(handle, tray::worst_remaining(&providers));
            }
            view_clone.borrow_mut().update_providers(providers);
        }
    });

    // Despierta al recolector cuando cambia la config de providers
    let refresh = Arc::new(Notify::new());

    let (app_clone, window_clone, refresh_clone) = (app.clone(), window.clone(), refresh.clone());
    glib::spawn_future_local(async move {
        while let Some(cmd) = tray_rx.recv().await {
            match cmd {
                tray::TrayCmd::Toggle => window_clone.set_visible(!window_clone.is_visible()),
                tray::TrayCmd::Settings => ui::settings::open(&app_clone, refresh_clone.clone()),
                tray::TrayCmd::Quit => app_clone.quit(),
            }
        }
    });

    // Hilo recolector en segundo plano
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(e) => {
                tracing::error!("Error iniciando Tokio runtime: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            loop {
                // Se relee cada vuelta para tomar los providers agregados en la configuración
                let config = WidgetConfig::load();
                tracing::info!("Consultando cuotas de uso...");
                let mut providers = Vec::new();

                // Solo se muestran las IAs instaladas (existe su carpeta de config en ~)
                let home = dirs::home_dir().unwrap_or_default();

                // 1. Consultar Codex (archivos locales)
                if home.join(".codex").exists() {
                    providers.push(data::codex::fetch_codex_usage());
                }

                // 2. Consultar Claude (API Anthropic + auto-login)
                if home.join(".claude").exists() {
                    providers.push(data::claude::fetch_claude_usage(config.auto_refresh_claude).await);
                }

                // 3. Providers agregados por el usuario (DeepSeek, OpenRouter, OpenCode)
                for p in &config.providers {
                    providers.push(data::fetch_configured(p).await);
                }

                if sender.send(providers).is_err() {
                    break;
                }

                let interval = std::time::Duration::from_secs(config.update_interval_secs);
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = refresh.notified() => {}
                }
            }
        });
    });

    window.present();
}
