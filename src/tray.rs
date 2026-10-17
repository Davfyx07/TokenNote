use ksni::blocking::TrayMethods;
use ksni::menu::StandardItem;
use tokio::sync::mpsc::UnboundedSender;

pub enum TrayCmd {
    Toggle,
    Quit,
}

struct AppTray {
    tx: UnboundedSender<TrayCmd>,
}

impl ksni::Tray for AppTray {
    fn id(&self) -> String {
        "ai-usage-widget".into()
    }

    fn title(&self) -> String {
        "AI Usage Widget".into()
    }

    fn icon_name(&self) -> String {
        "utilities-system-monitor".into()
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
pub fn spawn(tx: UnboundedSender<TrayCmd>) -> Option<ksni::blocking::Handle<impl ksni::Tray>> {
    AppTray { tx }
        .spawn()
        .map_err(|e| tracing::warn!("No se pudo crear el icono de bandeja: {}", e))
        .ok()
}
