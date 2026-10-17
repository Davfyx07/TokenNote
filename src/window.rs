use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider, GestureDrag, gdk};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::config::WidgetConfig;

const STYLES_CSS: &str = r#"
window.overlay {
    background-color: transparent;
}

#widget-root {
    background-color: rgba(18, 20, 28, 0.78);
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 18px;
    padding: 14px 18px;
}

#provider-title {
    font-weight: 700;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.95);
}

#short-limit-label {
    color: #ffffff;
    margin-top: 1px;
}

#weekly-limit-label {
    color: rgba(255, 255, 255, 0.75);
    margin-top: 2px;
}
"#;

pub fn create_overlay_window(app: &Application, config: &WidgetConfig) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("AI Usage Monitor")
        .decorated(false)
        .build();
    window.add_css_class("overlay");

    // Configuración Layer-Shell
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::None);

    // Anclaje arriba a la derecha
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Right, true);
    window.set_margin(Edge::Top, config.margin_top);
    window.set_margin(Edge::Right, config.margin_right);

    enable_drag(&window, config);
    load_css();
    window
}

/// Layer-shell no deja mover la ventana: arrastrar cambia los márgenes del anclaje.
fn enable_drag(window: &ApplicationWindow, config: &WidgetConfig) {
    let cfg = Rc::new(RefCell::new(config.clone()));
    let drag = GestureDrag::new();

    let (w, c) = (window.clone(), cfg.clone());
    drag.connect_drag_update(move |_, dx, dy| {
        // El offset es relativo a la ventana, que se mueve con el cursor: se acumula
        let mut c = c.borrow_mut();
        c.margin_top = (c.margin_top + dy as i32).max(0);
        c.margin_right = (c.margin_right - dx as i32).max(0);
        w.set_margin(Edge::Top, c.margin_top);
        w.set_margin(Edge::Right, c.margin_right);
    });

    drag.connect_drag_end(move |_, _, _| {
        // Releer para no pisar cambios hechos desde la ventana de configuración
        let mut fresh = WidgetConfig::load();
        fresh.margin_top = cfg.borrow().margin_top;
        fresh.margin_right = cfg.borrow().margin_right;
        if let Err(e) = fresh.save() {
            tracing::warn!("No se pudo guardar la posición: {}", e);
        }
    });

    window.add_controller(drag);
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(STYLES_CSS);

    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
