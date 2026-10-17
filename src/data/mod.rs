use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::ProviderConfig;

/// Representa una métrica de límite de tokens o cuota (ej: 5 horas o Semanal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLimit {
    /// Etiqueta corta que se muestra junto al %: "5h", "sem", "$4.20", "12k tok"
    pub name: String,
    
    /// Porcentaje consumido (0.0 a 100.0)
    pub used_percent: f64,
    
    /// Porcentaje restante disponible (100.0 - used_percent)
    pub remaining_percent: f64,
    
    /// Timestamp Unix en segundos de cuándo se reinicia la cuota
    pub resets_at: Option<i64>,
    
    /// Texto amigable para humanos del tiempo hasta el reinicio (ej: "3h 45m" o "5d 12h")
    pub reset_in_human: String,
}

/// Representa el estado consolidado de una IA instalada en el sistema (Codex, Claude, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderUsage {
    /// Identificador único ("codex", "claude")
    pub provider_id: String,
    
    /// Nombre para mostrar ("Codex", "Claude")
    pub display_name: String,
    
    /// Límite de corto plazo / sesión (5 horas) -> Representado por el ARO circular
    pub short_limit: Option<UsageLimit>,
    
    /// Límite de largo plazo (semanal) -> Representado por la BARRA horizontal
    pub weekly_limit: Option<UsageLimit>,
    
    /// Indica si se pudo leer la información correctamente
    pub is_available: bool,
    
    /// Mensaje de error o estado si no está disponible (ej: "No logueado", "Token expirado")
    pub status_message: Option<String>,
}

impl AiProviderUsage {
    /// Estado "no disponible" con un mensaje para mostrar en la tarjeta
    pub fn error(provider_id: &str, display_name: &str, message: String) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            display_name: display_name.to_string(),
            short_limit: None,
            weekly_limit: None,
            is_available: false,
            status_message: Some(message),
        }
    }
}

pub mod claude;
pub mod codex;
pub mod deepseek;
pub mod opencode;
pub mod openrouter;

/// Consulta un provider agregado desde la configuración
pub async fn fetch_configured(p: &ProviderConfig) -> AiProviderUsage {
    match p.kind.as_str() {
        "deepseek" => deepseek::fetch_deepseek_usage(&p.api_key, p.budget).await,
        "openrouter" => openrouter::fetch_openrouter_usage(&p.api_key).await,
        "opencode" => opencode::fetch_opencode_usage(p.budget),
        other => AiProviderUsage::error(other, p.display_name(), "Tipo desconocido".to_string()),
    }
}

/// Convierte un timestamp Unix en segundos a una cadena legible de tiempo restante
/// Por ejemplo: "5d 12h", "3h 45m", o "Listo" si ya pasó la fecha de reinicio.
pub fn format_duration_until(epoch_secs: i64) -> String {
    let now = Utc::now().timestamp();
    let diff = epoch_secs - now;

    if diff <= 0 {
        return "Listo".to_string();
    }

    let days = diff / 86400;
    let hours = (diff % 86400) / 3600;
    let minutes = (diff % 3600) / 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes.max(1))
    }
}
