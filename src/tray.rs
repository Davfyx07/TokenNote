use ksni::blocking::TrayMethods;
use ksni::menu::StandardItem;
use std::f64::consts::PI;
use tokio::sync::mpsc::UnboundedSender;

use crate::data::AiProviderUsage;
use crate::ui::ring::draw_usage_ring;

pub enum TrayCmd {
    Toggle,
    Quit,
}

pub struct AppTray {
    tx: UnboundedSender<TrayCmd>,
    icons: Vec<ksni::Icon>,
}

impl ksni::Tray for AppTray {
    fn id(&self) -> String {
        "ai-usage-widget".into()
    }

    fn title(&self) -> String {
        "AI Usage Widget".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icons.clone()
    }

    // Clic izquierdo: mostrar/ocultar
    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayCmd::Toggle);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Mostrar/Ocultar".into(),
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send(TrayCmd::Toggle);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Salir".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send(TrayCmd::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Registra el icono en la bandeja (StatusNotifierItem). Hay que mantener vivo el handle.
pub fn spawn(tx: UnboundedSender<TrayCmd>) -> Option<ksni::blocking::Handle<AppTray>> {
    let icons = logo_icons(None);
    AppTray { tx, icons }
        .spawn()
        .map_err(|e| tracing::warn!("No se pudo crear el icono de bandeja: {}", e))
        .ok()
}

/// Redibuja el aro del icono con el % restante (None = sin datos, aro vacío)
pub fn set_level(handle: &ksni::blocking::Handle<AppTray>, percent: Option<f64>) {
    let icons = logo_icons(percent);
    handle.update(|t| t.icons = icons);
}

/// % restante más bajo entre todos los límites de todas las IAs disponibles
pub fn worst_remaining(providers: &[AiProviderUsage]) -> Option<f64> {
    providers
        .iter()
        .filter(|p| p.is_available)
        .flat_map(|p| [&p.short_limit, &p.weekly_limit])
        .flatten()
        .map(|l| l.remaining_percent)
        .reduce(f64::min)
}

fn logo_icons(percent: Option<f64>) -> Vec<ksni::Icon> {
    vec![logo_icon(32, percent), logo_icon(64, percent)]
}

/// Logo: el mismo aro del widget sobre un disco oscuro, con una "T" de token al centro
fn draw_logo(cr: &cairo::Context, percent: Option<f64>) {
    cr.set_source_rgba(0.07, 0.08, 0.11, 0.95);
    cr.arc(32.0, 32.0, 31.0, 0.0, 2.0 * PI);
    let _ = cr.fill();

    draw_usage_ring(cr, percent.unwrap_or(0.0), 64.0, 64.0);

    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(28.0);
    if let Ok(ext) = cr.text_extents("T") {
        cr.move_to(32.0 - ext.width() / 2.0 - ext.x_bearing(), 32.0 - ext.height() / 2.0 - ext.y_bearing());
        let _ = cr.show_text("T");
    }
}

fn logo_icon(size: i32, percent: Option<f64>) -> ksni::Icon {
    let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, size, size)
        .expect("no se pudo crear la superficie del logo");
    if let Ok(cr) = cairo::Context::new(&surface) {
        cr.scale(size as f64 / 64.0, size as f64 / 64.0);
        draw_logo(&cr, percent);
    }
    let stride = surface.stride() as usize;
    let raw = surface.data().expect("superficie del logo en uso");
    icon_from_cairo(&raw, size, stride)
}

/// Cairo: BGRA premultiplicado (little-endian) -> SNI: ARGB sin premultiplicar (big-endian)
fn icon_from_cairo(raw: &[u8], size: i32, stride: usize) -> ksni::Icon {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for row in raw.chunks(stride).take(size as usize) {
        for px in row[..size as usize * 4].chunks_exact(4) {
            let a = px[3];
            let un = |c: u8| if a == 0 { 0 } else { (c as u32 * 255 / a as u32) as u8 };
            data.extend_from_slice(&[a, un(px[2]), un(px[1]), un(px[0])]);
        }
    }
    ksni::Icon { width: size, height: size, data }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::UsageLimit;

    #[test]
    fn convierte_bgra_premultiplicado_a_argb() {
        // 1 px rojo al 50%: BGRA premultiplicado = [0, 0, 128, 128]
        let icon = icon_from_cairo(&[0, 0, 128, 128], 1, 4);
        assert_eq!(icon.data, vec![128, 255, 0, 0]);
        assert_eq!(logo_icon(32, Some(50.0)).data.len(), 32 * 32 * 4);
    }

    #[test]
    fn worst_remaining_toma_el_minimo_de_las_disponibles() {
        let limit = |r: f64| Some(UsageLimit {
            name: String::new(), used_percent: 100.0 - r, remaining_percent: r,
            resets_at: None, reset_in_human: String::new(),
        });
        let prov = |avail, s, w| AiProviderUsage {
            provider_id: String::new(), display_name: String::new(),
            short_limit: s, weekly_limit: w, is_available: avail, status_message: None,
        };
        assert_eq!(worst_remaining(&[]), None);
        assert_eq!(worst_remaining(&[prov(true, limit(80.0), limit(30.0)), prov(true, limit(60.0), None)]), Some(30.0));
        assert_eq!(worst_remaining(&[prov(false, limit(5.0), None), prov(true, limit(90.0), None)]), Some(90.0));
    }
}
