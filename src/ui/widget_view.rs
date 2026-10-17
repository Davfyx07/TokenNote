use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;
use gtk4::{gdk, glib, Box as GtkBox, DrawingArea, Image, Label, Orientation, Overlay};

use crate::data::AiProviderUsage;
use super::ring::draw_usage_ring;
use super::bar::draw_usage_bar;

/// Estructura que mantiene las referencias a los widgets GTK de una IA para actualizarlos dinámicamente
pub struct ProviderCardWidgets {
    pub container: GtkBox,
    pub ring_area: DrawingArea,
    pub short_label: Label,
    pub bar_area: DrawingArea,
    pub weekly_label: Label,
    pub data: Rc<RefCell<AiProviderUsage>>,
}

impl ProviderCardWidgets {
    pub fn new(initial_data: AiProviderUsage) -> Self {
        let data = Rc::new(RefCell::new(initial_data));

        // Contenedor vertical de la columna de esta IA
        let container = GtkBox::new(Orientation::Vertical, 6);
        container.set_widget_name("provider-card");
        container.set_width_request(110);

        // 1. Título con el nombre de la IA (ej: "Codex", "Claude")
        let title_label = Label::new(Some(&data.borrow().display_name));
        title_label.set_widget_name("provider-title");

        // 2. Área de dibujo del Aro circular (Límite 5h / Sesión)
        let ring_area = DrawingArea::new();
        ring_area.set_content_width(70);
        ring_area.set_content_height(70);
        ring_area.set_halign(gtk4::Align::Center);

        let data_ring = data.clone();
        ring_area.set_draw_func(move |_, cr, width, height| {
            let current = data_ring.borrow();
            let percent = current
                .short_limit
                .as_ref()
                .map(|l| l.remaining_percent)
                .unwrap_or(0.0);
            draw_usage_ring(cr, percent, width as f64, height as f64);
        });

        // Logo de la IA en el centro del aro (embebido en el binario)
        let logo: &'static [u8] = match data.borrow().provider_id.as_str() {
            "claude" => include_bytes!("../../assets/claude.png"),
            _ => include_bytes!("../../assets/gpt.png"),
        };
        let ring_overlay = Overlay::new();
        ring_overlay.set_halign(gtk4::Align::Center);
        ring_overlay.set_child(Some(&ring_area));
        match gdk::Texture::from_bytes(&glib::Bytes::from_static(logo)) {
          Err(e) => tracing::warn!("No se pudo cargar el logo: {}", e),
          Ok(texture) => {
            let icon = Image::from_paintable(Some(&texture));
            icon.set_pixel_size(28);
            icon.set_halign(gtk4::Align::Center);
            icon.set_valign(gtk4::Align::Center);
            ring_overlay.add_overlay(&icon);
          }
        }

        // 3. Etiqueta informativa del límite de 5h
        let short_label = Label::new(None);
        short_label.set_widget_name("short-limit-label");

        // 4. Área de dibujo de la Barra horizontal (Límite semanal)
        let bar_area = DrawingArea::new();
        bar_area.set_content_width(85);
        bar_area.set_content_height(8);
        bar_area.set_halign(gtk4::Align::Center);

        let data_bar = data.clone();
        bar_area.set_draw_func(move |_, cr, width, height| {
            let current = data_bar.borrow();
            let percent = current
                .weekly_limit
                .as_ref()
                .map(|l| l.remaining_percent)
                .unwrap_or(0.0);
            draw_usage_bar(cr, percent, width as f64, height as f64);
        });

        // 5. Etiqueta informativa del límite semanal
        let weekly_label = Label::new(None);
        weekly_label.set_widget_name("weekly-limit-label");

        // Empaquetar todos los elementos en la tarjeta
        container.append(&title_label);
        container.append(&ring_overlay);
        container.append(&short_label);
        container.append(&bar_area);
        container.append(&weekly_label);

        let card = Self {
            container,
            ring_area,
            short_label,
            bar_area,
            weekly_label,
            data,
        };

        card.update_view();
        card
    }

    /// Actualiza los textos y solicita el redibujado de los canvas con los datos actuales
    pub fn update_data(&self, new_data: AiProviderUsage) {
        *self.data.borrow_mut() = new_data;
        self.update_view();
    }

    fn update_view(&self) {
        let current = self.data.borrow();

        // Si no está disponible, mostrar el mensaje de error o estado
        if !current.is_available {
            let msg = current
                .status_message
                .as_deref()
                .unwrap_or("No disponible");
            self.short_label.set_text(msg);
            self.weekly_label.set_text("");
            self.ring_area.queue_draw();
            self.bar_area.queue_draw();
            return;
        }

        // Texto del límite corto (5h)
        if let Some(short) = &current.short_limit {
            self.short_label.set_markup(&format!(
                "<span size='small' weight='bold'>{:.0}%</span> <span size='x-small' alpha='70%'>5h</span>",
                short.remaining_percent
            ));
        } else {
            self.short_label.set_text("—");
        }

        // Texto del límite semanal
        if let Some(weekly) = &current.weekly_limit {
            self.weekly_label.set_markup(&format!(
                "<span size='x-small' alpha='85%'>{:.0}% sem · {}</span>",
                weekly.remaining_percent, weekly.reset_in_human
            ));
        } else {
            self.weekly_label.set_text("—");
        }

        // Solicitar a GTK que vuelva a invocar draw_func
        self.ring_area.queue_draw();
        self.bar_area.queue_draw();
    }
}

/// Contenedor principal del widget que aloja las tarjetas de todas las IAs lado a lado
pub struct MainWidgetView {
    pub root_container: GtkBox,
    pub cards: Vec<ProviderCardWidgets>,
}

impl MainWidgetView {
    pub fn new() -> Self {
        let root_container = GtkBox::new(Orientation::Horizontal, 16);
        root_container.set_widget_name("widget-root");

        Self {
            root_container,
            cards: Vec::new(),
        }
    }

    /// Actualiza o inicializa las tarjetas con la lista de proveedores
    pub fn update_providers(&mut self, providers: Vec<AiProviderUsage>) {
        // Si cambió qué IAs hay (ej: el usuario instaló una), reconstruimos la lista
        let same_providers = self.cards.len() == providers.len()
            && self.cards.iter().zip(&providers)
                .all(|(card, p)| card.data.borrow().provider_id == p.provider_id);
        if !same_providers {
            // Limpiar hijos anteriores
            while let Some(child) = self.root_container.first_child() {
                self.root_container.remove(&child);
            }
            self.cards.clear();

            for provider in providers {
                let card = ProviderCardWidgets::new(provider);
                self.root_container.append(&card.container);
                self.cards.push(card);
            }
        } else {
            // Actualizar datos existentes
            for (card, data) in self.cards.iter().zip(providers.into_iter()) {
                card.update_data(data);
            }
        }
    }
}
