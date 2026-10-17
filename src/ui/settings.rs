use std::sync::Arc;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, Box as GtkBox, Button, DropDown, Label, ListBox, Orientation,
    PasswordEntry, SelectionMode, Separator, SpinButton, Window,
};
use tokio::sync::Notify;

use crate::config::{ProviderConfig, WidgetConfig, PROVIDER_KINDS};

const SETTINGS_NAME: &str = "settings-window";

/// Abre (o enfoca) la ventana para agregar y quitar providers.
/// `refresh` despierta al hilo recolector para que use la config nueva sin esperar el intervalo.
pub fn open(app: &Application, refresh: Arc<Notify>) {
    if let Some(existing) = app.windows().into_iter().find(|w| w.widget_name() == SETTINGS_NAME) {
        existing.present();
        return;
    }

    let window = Window::builder()
        .application(app)
        .title("Configuración · AI Usage Widget")
        .default_width(420)
        .build();
    window.set_widget_name(SETTINGS_NAME);

    let root = GtkBox::new(Orientation::Vertical, 10);
    for set_margin in [GtkBox::set_margin_top, GtkBox::set_margin_bottom, GtkBox::set_margin_start, GtkBox::set_margin_end] {
        set_margin(&root, 16);
    }

    let title = Label::new(None);
    title.set_markup("<b>Providers</b>");
    title.set_halign(Align::Start);

    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::None);
    list.add_css_class("boxed-list");

    let add_title = Label::new(None);
    add_title.set_markup("<b>Agregar provider</b>");
    add_title.set_halign(Align::Start);

    let names: Vec<&str> = PROVIDER_KINDS.iter().map(|(_, name)| *name).collect();
    let kind_drop = DropDown::from_strings(&names);

    let key_entry = PasswordEntry::new();
    key_entry.set_show_peek_icon(true);
    key_entry.set_property("placeholder-text", "API key");

    let budget_label = Label::new(None);
    budget_label.set_halign(Align::Start);
    budget_label.set_hexpand(true);
    let budget_spin = SpinButton::with_range(1.0, 1_000_000_000.0, 1.0);
    let budget_row = GtkBox::new(Orientation::Horizontal, 8);
    budget_row.append(&budget_label);
    budget_row.append(&budget_spin);

    let hint = Label::new(None);
    hint.set_wrap(true);
    hint.set_halign(Align::Start);
    hint.add_css_class("dim-label");

    let add_button = Button::with_label("Agregar");
    add_button.set_halign(Align::End);

    // Ajustar el formulario al tipo elegido
    let update_form = {
        let (key_entry, budget_row, budget_label, budget_spin, hint) =
            (key_entry.clone(), budget_row.clone(), budget_label.clone(), budget_spin.clone(), hint.clone());
        move |kind: &str| {
            key_entry.set_visible(kind != "opencode");
            budget_row.set_visible(kind != "openrouter");
            match kind {
                "deepseek" => {
                    budget_label.set_text("Saldo de referencia (USD) = 100%");
                    budget_spin.set_increments(1.0, 10.0);
                    budget_spin.set_value(10.0);
                    hint.set_text("El aro muestra tu saldo frente a este monto.");
                }
                "opencode" => {
                    budget_label.set_text("Tokens por día = 100%");
                    budget_spin.set_increments(10_000.0, 100_000.0);
                    budget_spin.set_value(1_000_000.0);
                    hint.set_text("Lee ~/.local/share/opencode/opencode.db; no necesita API key.");
                }
                _ => hint.set_text("Usa el límite de crédito de la key; si no tiene límite no hay porcentaje."),
            }
        }
    };
    update_form(PROVIDER_KINDS[0].0);
    let update_form_clone = update_form.clone();
    kind_drop.connect_selected_notify(move |d| update_form_clone(PROVIDER_KINDS[d.selected() as usize].0));

    {
        let (list, kind_drop, key_entry, budget_spin, hint, refresh) =
            (list.clone(), kind_drop.clone(), key_entry.clone(), budget_spin.clone(), hint.clone(), refresh.clone());
        add_button.connect_clicked(move |_| {
            let kind = PROVIDER_KINDS[kind_drop.selected() as usize].0;
            let api_key = key_entry.text().trim().to_string();
            if kind != "opencode" && api_key.is_empty() {
                hint.set_text("Falta la API key.");
                return;
            }
            let mut cfg = WidgetConfig::load();
            cfg.providers.push(ProviderConfig {
                kind: kind.to_string(),
                api_key: if kind == "opencode" { String::new() } else { api_key },
                budget: budget_spin.value(),
            });
            match cfg.save() {
                Ok(()) => {
                    key_entry.set_text("");
                    hint.set_text("Agregado.");
                    refresh.notify_one();
                    fill_list(&list, refresh.clone());
                }
                Err(e) => hint.set_text(&format!("No se pudo guardar: {}", e)),
            }
        });
    }

    fill_list(&list, refresh);

    root.append(&title);
    root.append(&list);
    root.append(&Separator::new(Orientation::Horizontal));
    root.append(&add_title);
    root.append(&kind_drop);
    root.append(&key_entry);
    root.append(&budget_row);
    root.append(&hint);
    root.append(&add_button);
    window.set_child(Some(&root));
    window.present();
}

/// Rellena la lista con los providers guardados, cada uno con su botón "Quitar"
fn fill_list(list: &ListBox, refresh: Arc<Notify>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let providers = WidgetConfig::load().providers;

    if providers.is_empty() {
        let empty = Label::new(Some("Ninguno todavía (Claude y Codex se detectan solos)"));
        empty.add_css_class("dim-label");
        empty.set_margin_top(8);
        empty.set_margin_bottom(8);
        list.append(&empty);
    }

    for (i, p) in providers.iter().enumerate() {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.set_margin_top(6);
        row.set_margin_bottom(6);
        row.set_margin_start(8);
        row.set_margin_end(8);

        let label = Label::new(Some(&describe(p)));
        label.set_halign(Align::Start);
        label.set_hexpand(true);

        let remove = Button::with_label("Quitar");
        let (list_clone, refresh) = (list.clone(), refresh.clone());
        remove.connect_clicked(move |_| {
            let mut cfg = WidgetConfig::load();
            if i < cfg.providers.len() {
                cfg.providers.remove(i);
                if let Err(e) = cfg.save() {
                    tracing::warn!("No se pudo guardar la config: {}", e);
                }
            }
            refresh.notify_one();
            fill_list(&list_clone, refresh.clone());
        });

        row.append(&label);
        row.append(&remove);
        list.append(&row);
    }
}

/// "DeepSeek · ••••a1b2 · ref $10" — nunca muestra la key completa
fn describe(p: &ProviderConfig) -> String {
    let mut parts = vec![p.display_name().to_string()];
    if !p.api_key.is_empty() {
        let tail: String = p.api_key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
        parts.push(format!("••••{}", tail));
    }
    match p.kind.as_str() {
        "deepseek" => parts.push(format!("ref ${:.0}", p.budget)),
        "opencode" => parts.push(format!("{:.0} tok/día", p.budget)),
        _ => {}
    }
    parts.join(" · ")
}
