use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use serde::Deserialize;
use chrono::Utc;

use super::{AiProviderUsage, UsageLimit, format_duration_until};

/// Estructura interna para deserializar el fragmento de rate_limits del JSONL de Codex
#[derive(Debug, Deserialize)]
struct CodexSessionEvent {
    payload: Option<CodexPayload>,
}

#[derive(Debug, Deserialize)]
struct CodexPayload {
    rate_limits: Option<CodexRateLimits>,
}

#[derive(Debug, Deserialize)]
struct CodexRateLimits {
    primary: Option<CodexWindowLimit>,
    secondary: Option<CodexWindowLimit>,
}

#[derive(Debug, Deserialize)]
struct CodexWindowLimit {
    used_percent: f64,
    resets_at: Option<i64>,
}

/// Encuentra el archivo de sesión .jsonl más recientemente modificado en ~/.codex/sessions/
fn find_latest_session_file() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let sessions_dir = home.join(".codex").join("sessions");
    
    if !sessions_dir.exists() {
        return None;
    }

    let mut latest_file: Option<(PathBuf, std::time::SystemTime)> = None;

    for entry in WalkDir::new(sessions_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "jsonl"))
    {
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                match &latest_file {
                    None => latest_file = Some((entry.path().to_path_buf(), modified)),
                    Some((_, latest_time)) if modified > *latest_time => {
                        latest_file = Some((entry.path().to_path_buf(), modified));
                    }
                    _ => {}
                }
            }
        }
    }

    latest_file.map(|(path, _)| path)
}

/// Lee las últimas líneas del archivo .jsonl buscando el evento más reciente con rate_limits
fn parse_rate_limits_from_file(path: &Path) -> Result<CodexRateLimits, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut last_rate_limits: Option<CodexRateLimits> = None;

    // Leemos línea por línea. Como el archivo puede tener miles de líneas,
    // filtramos solo aquellas que contengan la subcadena "rate_limits" para que sea ultra rápido.
    for line_result in reader.lines() {
        if let Ok(line) = line_result {
            if line.contains("rate_limits") {
                if let Ok(event) = serde_json::from_str::<CodexSessionEvent>(&line) {
                    if let Some(payload) = event.payload {
                        if let Some(limits) = payload.rate_limits {
                            last_rate_limits = Some(limits);
                        }
                    }
                }
            }
        }
    }

    last_rate_limits.ok_or_else(|| "No se encontró ningún evento rate_limits en la sesión".into())
}

/// Función principal expuesta: obtiene el uso de Codex desde el sistema local
pub fn fetch_codex_usage() -> AiProviderUsage {
    let session_file = match find_latest_session_file() {
        Some(path) => path,
        None => {
            return AiProviderUsage {
                provider_id: "codex".to_string(),
                display_name: "Codex".to_string(),
                short_limit: None,
                weekly_limit: None,
                is_available: false,
                status_message: Some("No se encontraron sesiones (~/.codex/sessions)".to_string()),
            };
        }
    };

    match parse_rate_limits_from_file(&session_file) {
        Ok(limits) => {
            let now = Utc::now().timestamp();

            // Límite de 5 horas (Primary rolling window) -> para el Aro circular
            let short_limit = limits.primary.map(|p| {
                let resets = p.resets_at.unwrap_or(0);
                // Si el tiempo de reseteo ya pasó, el límite disponible es el 100%
                let remaining = if resets <= now && resets > 0 {
                    100.0
                } else {
                    (100.0 - p.used_percent).clamp(0.0, 100.0)
                };

                UsageLimit {
                    name: "Sesión 5h".to_string(),
                    used_percent: p.used_percent,
                    remaining_percent: remaining,
                    resets_at: p.resets_at,
                    reset_in_human: format_duration_until(resets),
                }
            });

            // Límite semanal (Secondary rolling window) -> para la Barra horizontal
            let weekly_limit = limits.secondary.map(|s| {
                let resets = s.resets_at.unwrap_or(0);
                let remaining = if resets <= now && resets > 0 {
                    100.0
                } else {
                    (100.0 - s.used_percent).clamp(0.0, 100.0)
                };

                UsageLimit {
                    name: "Semanal".to_string(),
                    used_percent: s.used_percent,
                    remaining_percent: remaining,
                    resets_at: s.resets_at,
                    reset_in_human: format_duration_until(resets),
                }
            });

            AiProviderUsage {
                provider_id: "codex".to_string(),
                display_name: "Codex".to_string(),
                short_limit,
                weekly_limit,
                is_available: true,
                status_message: None,
            }
        }
        Err(e) => AiProviderUsage {
            provider_id: "codex".to_string(),
            display_name: "Codex".to_string(),
            short_limit: None,
            weekly_limit: None,
            is_available: false,
            status_message: Some(format!("Error leyendo límites: {}", e)),
        },
    }
}
