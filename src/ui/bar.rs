use cairo::Context;
use super::ring::get_status_color;

/// Dibuja una barra de progreso horizontal con esquinas redondeadas (estilo cápsula)
pub fn draw_usage_bar(
    cr: &Context,
    percent_remaining: f64,
    width: f64,
    height: f64,
) {
    let radius = height / 2.0;

    // Función auxiliar para dibujar un rectángulo con bordes redondeados
    let rounded_rect = |cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64| {
        let r = r.min(w / 2.0).min(h / 2.0);
        cr.new_sub_path();
        cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
        cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
        cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
        cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
        cr.close_path();
    };

    // 1. Dibujar el fondo de la barra
    rounded_rect(cr, 0.0, 0.0, width, height, radius);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12);
    let _ = cr.fill();

    // 2. Dibujar el progreso activo
    let clamped = percent_remaining.clamp(0.0, 100.0);
    if clamped > 0.0 {
        let active_width = (width * (clamped / 100.0)).max(radius * 2.0).min(width);
        rounded_rect(cr, 0.0, 0.0, active_width, height, radius);
        
        let (r, g, b) = get_status_color(clamped);
        cr.set_source_rgb(r, g, b);
        let _ = cr.fill();
    }
}
