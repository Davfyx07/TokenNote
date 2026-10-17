use serde::Deserialize;

use super::{AiProviderUsage, UsageLimit};

#[derive(Debug, Deserialize)]
struct KeyResponse {
    data: KeyInfo,
}

#[derive(Debug, Deserialize)]
struct KeyInfo {
    usage: f64,
    limit: Option<f64>,
}

/// Uso y límite de crédito de la API key de OpenRouter
pub async fn fetch_openrouter_usage(api_key: &str) -> AiProviderUsage {
    let res = reqwest::Client::new()
        .get("https://openrouter.ai/api/v1/key")
        .bearer_auth(api_key)
        .send()
        .await;

    match res {
        Ok(resp) if resp.status().is_success() => match resp.json::<KeyResponse>().await {
            Ok(data) => usage_from_key(&data.data),
            Err(e) => AiProviderUsage::error("openrouter", "OpenRouter", format!("Error parseando API: {}", e)),
        },
        Ok(resp) => AiProviderUsage::error("openrouter", "OpenRouter", format!("HTTP Error {}", resp.status())),
        Err(e) => AiProviderUsage::error("openrouter", "OpenRouter", format!("Sin conexión: {}", e)),
    }
}

fn usage_from_key(info: &KeyInfo) -> AiProviderUsage {
    let Some(limit) = info.limit.filter(|l| *l > 0.0) else {
        // Sin límite no hay porcentaje que mostrar
        return AiProviderUsage::error(
            "openrouter",
            "OpenRouter",
            format!("${:.2} usado · sin límite", info.usage),
        );
    };
    let left = (limit - info.usage).max(0.0);
    let remaining = (left / limit * 100.0).clamp(0.0, 100.0);

    AiProviderUsage {
        provider_id: "openrouter".to_string(),
        display_name: "OpenRouter".to_string(),
        short_limit: Some(UsageLimit {
            name: format!("${:.2}", left),
            used_percent: 100.0 - remaining,
            remaining_percent: remaining,
            resets_at: None,
            reset_in_human: String::new(),
        }),
        weekly_limit: None,
        is_available: true,
        status_message: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcentaje_con_y_sin_limite() {
        let json = r#"{"data":{"label":"k","usage":2.5,"limit":10,"is_free_tier":false}}"#;
        let data: KeyResponse = serde_json::from_str(json).unwrap();
        let limit = usage_from_key(&data.data).short_limit.unwrap();
        assert_eq!(limit.remaining_percent, 75.0);
        assert_eq!(limit.name, "$7.50");

        let json = r#"{"data":{"label":"k","usage":1.0,"limit":null}}"#;
        let data: KeyResponse = serde_json::from_str(json).unwrap();
        assert!(!usage_from_key(&data.data).is_available);
    }
}
