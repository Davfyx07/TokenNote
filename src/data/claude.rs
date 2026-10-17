use std::fs::File;
use std::io::BufReader;
use std::process::Command;
use serde::Deserialize;
use chrono::Utc;

use super::{AiProviderUsage, UsageLimit, format_duration_until};

#[derive(Debug, Deserialize)]
struct ClaudeCredentials {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<ClaudeOAuthConfig>,
}

#[derive(Debug, Deserialize, Clone)]
struct ClaudeOAuthConfig {
    #[serde(rename = "accessToken")]
    access_token: String,
    
    #[serde(rename = "expiresAt")]
    expires_at: Option<i64>, // En milisegundos Unix
}

#[derive(Debug, Deserialize)]
struct ClaudeUsageResponse {
    limits: Option<Vec<ClaudeLimitItem>>,
}

#[derive(Debug, Deserialize)]
struct ClaudeLimitItem {
    kind: String,
    percent: Option<f64>,
    resets_at: Option<String>, // ISO 8601, ej: "2026-10-17T06:50:00+00:00"
}

/// Lee las credenciales de ~/.claude/.credentials.json
fn read_credentials() -> Option<ClaudeOAuthConfig> {
    let home = dirs::home_dir()?;
    let path = home.join(".claude").join(".credentials.json");
    
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let creds: ClaudeCredentials = serde_json::from_reader(reader).ok()?;
    
    creds.claude_ai_oauth
}

/// Intenta renovar el token ejecutando un prompt mínimo con Haiku en segundo plano
fn try_refresh_token() -> bool {
    tracing::info!("Intentando auto-renovar token de Claude ejecutando 'claude -p'...");
    
    let status = Command::new("claude")
        .args([
            "-p", "ok",
            "--model", "haiku",
            "--strict-mcp-config",
            "--mcp-config", "{\"mcpServers\":{}}"
        ])
        .output();

    match status {
        Ok(output) if output.status.success() => {
            tracing::info!("Token de Claude renovado exitosamente.");
            true
        }
        Ok(output) => {
            tracing::warn!(
                "El comando de refresh de Claude terminó con error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            false
        }
        Err(e) => {
            tracing::error!("No se pudo ejecutar el comando claude: {}", e);
            false
        }
    }
}

/// Obtiene el uso de Claude Code haciendo una petición a la API de Anthropic
pub async fn fetch_claude_usage(auto_refresh: bool) -> AiProviderUsage {
    let mut oauth = match read_credentials() {
        Some(config) => config,
        None => {
            return AiProviderUsage {
                provider_id: "claude".to_string(),
                display_name: "Claude".to_string(),
                short_limit: None,
                weekly_limit: None,
                is_available: false,
                status_message: Some("Ejecuta 'claude auth login'".to_string()),
            };
        }
    };

    // Verificar si el token expiró o está a punto de expirar (< 5 minutos)
    let now_millis = Utc::now().timestamp_millis();
    if let Some(exp) = oauth.expires_at {
        if exp <= now_millis + (5 * 60 * 1000) {
            if auto_refresh {
                if try_refresh_token() {
                    // Releer credenciales tras el refresh
                    if let Some(fresh_oauth) = read_credentials() {
                        oauth = fresh_oauth;
                    }
                }
            }
        }
    }

    // Consultar la API de Anthropic
    let client = reqwest::Client::new();
    let res = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {}", oauth.access_token))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", "claude-code/ai-usage-widget")
        .send()
        .await;

    match res {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<ClaudeUsageResponse>().await {
                Ok(data) => {
                    let mut weekly_limit = None;
                    let mut short_limit = None;

                    if let Some(limits) = data.limits {
                        for item in limits {
                            let used = item.percent.unwrap_or(0.0);
                            let remaining = (100.0 - used).clamp(0.0, 100.0);
                            let resets_at = item.resets_at.as_deref()
                                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                                .map(|d| d.timestamp());
                            let resets = resets_at.unwrap_or(0);
                            let human_time = format_duration_until(resets);

                            if item.kind == "weekly_all" {
                                weekly_limit = Some(UsageLimit {
                                    name: "Semanal".to_string(),
                                    used_percent: used,
                                    remaining_percent: remaining,
                                    resets_at,
                                    reset_in_human: human_time,
                                });
                            } else if item.kind.contains("session") || item.kind.contains("5h") || item.kind.contains("prompt") {
                                short_limit = Some(UsageLimit {
                                    name: "Sesión 5h".to_string(),
                                    used_percent: used,
                                    remaining_percent: remaining,
                                    resets_at,
                                    reset_in_human: human_time,
                                });
                            }
                        }
                    }

                    // Si Claude solo reporta cuota semanal y no reporta sesión,
                    // reflejamos la cuota disponible para el aro también o un 100% activo
                    if short_limit.is_none() && weekly_limit.is_some() {
                        let w = weekly_limit.as_ref().unwrap();
                        short_limit = Some(UsageLimit {
                            name: "Activo".to_string(),
                            used_percent: w.used_percent,
                            remaining_percent: w.remaining_percent,
                            resets_at: w.resets_at,
                            reset_in_human: w.reset_in_human.clone(),
                        });
                    }

                    AiProviderUsage {
                        provider_id: "claude".to_string(),
                        display_name: "Claude".to_string(),
                        short_limit,
                        weekly_limit,
                        is_available: true,
                        status_message: None,
                    }
                }
                Err(e) => AiProviderUsage {
                    provider_id: "claude".to_string(),
                    display_name: "Claude".to_string(),
                    short_limit: None,
                    weekly_limit: None,
                    is_available: false,
                    status_message: Some(format!("Error parseando API: {}", e)),
                },
            }
        }
        Ok(resp) => AiProviderUsage {
            provider_id: "claude".to_string(),
            display_name: "Claude".to_string(),
            short_limit: None,
            weekly_limit: None,
            is_available: false,
            status_message: Some(format!("HTTP Error {}", resp.status())),
        },
        Err(e) => AiProviderUsage {
            provider_id: "claude".to_string(),
            display_name: "Claude".to_string(),
            short_limit: None,
            weekly_limit: None,
            is_available: false,
            status_message: Some(format!("Sin conexión: {}", e)),
        },
    }
}
