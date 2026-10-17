use std::f64::consts::{FRAC_PI_2, PI};
use cairo::Context;

/// Retorna el color (R, G, B) correspondiente al porcentaje restante
pub fn get_status_color(percent: f64) -> (f64, f64, f64) {
    if percent > 40.0 {
        (0.29, 0.87, 0.50) // Verde esmeralda (#4ade80)
    } else if percent > 15.0 {
        (0.98, 0.80, 0.08) // Amarillo ámbar (#facc15)
    } else {
        (0.97, 0.44, 0.44) // Rojo coral (#f87171)
    }
}

/// Dibuja el aro de progreso circular de la sesión (5h) con Cairo
pub fn draw_usage_ring(
    cr: &Context,
    percent_remaining: f64,
    width: f64,
    height: f64,
) {
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let radius = (width.min(height) / 2.0) - 7.0;
    let line_width = 6.0;

    // 1. Dibujar el fondo del aro (anillo translúcido tenue)
    cr.set_line_width(line_width);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12);
    cr.arc(center_x, center_y, radius, 0.0, 2.0 * PI);
    let _ = cr.stroke();

    // 2. Dibujar el arco activo de progreso (en sentido horario desde arriba)
    let clamped_percent = percent_remaining.clamp(0.0, 100.0);
    if clamped_percent > 0.0 {
        let (r, g, b) = get_status_color(clamped_percent);
        cr.set_source_rgb(r, g, b);
        cr.set_line_cap(cairo::LineCap::Round);

        let start_angle = -FRAC_PI_2; // -90 grados (12:00)
        let sweep_angle = (clamped_percent / 100.0) * (2.0 * PI);
        cr.arc(center_x, center_y, radius, start_angle, start_angle + sweep_angle);
        let _ = cr.stroke();
    }
}
