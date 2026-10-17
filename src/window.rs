use gtk4::{Application, ApplicationWindow, CssProvider, gdk};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::config::WidgetConfig;

const STYLES_CSS: &str = r#"
window {
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

    // Configuración Layer-Shell
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::None);

    // Anclaje arriba a la derecha
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Right, true);
    window.set_margin(Edge::Top, config.margin_top);
    window.set_margin(Edge::Right, config.margin_right);

    load_css();
    window
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
