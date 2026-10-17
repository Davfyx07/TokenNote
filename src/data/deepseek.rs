use serde::Deserialize;

use super::{AiProviderUsage, UsageLimit};

#[derive(Debug, Deserialize)]
struct BalanceResponse {
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Debug, Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: String,
}

/// Saldo de la cuenta de DeepSeek; el aro es saldo / presupuesto de referencia
pub async fn fetch_deepseek_usage(api_key: &str, budget: f64) -> AiProviderUsage {
    let res = reqwest::Client::new()
        .get("https://api.deepseek.com/user/balance")
        .bearer_auth(api_key)
        .send()
        .await;

    match res {
        Ok(resp) if resp.status().is_success() => match resp.json::<BalanceResponse>().await {
            Ok(data) => usage_from_balance(&data, budget),
            Err(e) => AiProviderUsage::error("deepseek", "DeepSeek", format!("Error parseando API: {}", e)),
        },
        Ok(resp) => AiProviderUsage::error("deepseek", "DeepSeek", format!("HTTP Error {}", resp.status())),
        Err(e) => AiProviderUsage::error("deepseek", "DeepSeek", format!("Sin conexión: {}", e)),
    }
}

fn usage_from_balance(data: &BalanceResponse, budget: f64) -> AiProviderUsage {
    // Preferir USD si la cuenta tiene varias monedas
    let info = data
        .balance_infos
        .iter()
        .find(|b| b.currency == "USD")
        .or_else(|| data.balance_infos.first());

    let Some(info) = info else {
        return AiProviderUsage::error("deepseek", "DeepSeek", "Sin saldo".to_string());
    };
    let balance: f64 = info.total_balance.parse().unwrap_or(0.0);
    let symbol = if info.currency == "CNY" { "¥" } else { "$" };
    let remaining = if budget > 0.0 { (balance / budget * 100.0).clamp(0.0, 100.0) } else { 100.0 };

    AiProviderUsage {
        provider_id: "deepseek".to_string(),
        display_name: "DeepSeek".to_string(),
        short_limit: Some(UsageLimit {
            name: format!("{}{:.2}", symbol, balance),
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
    fn saldo_contra_presupuesto() {
        let json = r#"{"is_available":true,"balance_infos":[
            {"currency":"CNY","total_balance":"50.00","granted_balance":"0","topped_up_balance":"50.00"},
            {"currency":"USD","total_balance":"2.50","granted_balance":"0","topped_up_balance":"2.50"}]}"#;
        let data: BalanceResponse = serde_json::from_str(json).unwrap();
        let limit = usage_from_balance(&data, 10.0).short_limit.unwrap();
        assert_eq!(limit.remaining_percent, 25.0);
        assert_eq!(limit.name, "$2.50");
        // Con más saldo que el presupuesto el aro queda lleno
        assert_eq!(usage_from_balance(&data, 1.0).short_limit.unwrap().remaining_percent, 100.0);
    }
}
