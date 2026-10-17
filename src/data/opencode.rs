use chrono::{Duration, Local};
use rusqlite::{Connection, OpenFlags};

use super::{format_duration_until, AiProviderUsage, UsageLimit};

/// Tokens usados hoy en OpenCode (lee su sqlite local) contra un presupuesto diario
pub fn fetch_opencode_usage(daily_budget: f64) -> AiProviderUsage {
    let Some(db_path) = dirs::data_dir().map(|d| d.join("opencode").join("opencode.db")) else {
        return AiProviderUsage::error("opencode", "OpenCode", "Sin carpeta de datos".to_string());
    };
    let midnight = Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|t| t.and_local_timezone(Local).earliest());
    let Some(midnight) = midnight else {
        return AiProviderUsage::error("opencode", "OpenCode", "Hora local inválida".to_string());
    };

    let tokens = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .and_then(|conn| tokens_since(&conn, midnight.timestamp_millis()));

    match tokens {
        Ok(used) => {
            let remaining = if daily_budget > 0.0 {
                (100.0 - used / daily_budget * 100.0).clamp(0.0, 100.0)
            } else {
                100.0
            };
            let next_midnight = (midnight + Duration::days(1)).timestamp();
            AiProviderUsage {
                provider_id: "opencode".to_string(),
                display_name: "OpenCode".to_string(),
                short_limit: Some(UsageLimit {
                    name: format!("{} tok", format_tokens(used)),
                    used_percent: 100.0 - remaining,
                    remaining_percent: remaining,
                    resets_at: Some(next_midnight),
                    reset_in_human: format_duration_until(next_midnight),
                }),
                weekly_limit: None,
                is_available: true,
                status_message: None,
            }
        }
        Err(e) => AiProviderUsage::error("opencode", "OpenCode", format!("No se pudo leer opencode.db: {}", e)),
    }
}

/// Suma los tokens de las respuestas del asistente desde `since_ms`
fn tokens_since(conn: &Connection, since_ms: i64) -> rusqlite::Result<f64> {
    conn.query_row(
        "SELECT COALESCE(SUM(COALESCE(
             json_extract(data, '$.tokens.total'),
             json_extract(data, '$.tokens.input') + json_extract(data, '$.tokens.output'),
             0)), 0)
         FROM message
         WHERE time_created >= ?1 AND json_extract(data, '$.role') = 'assistant'",
        [since_ms],
        |row| row.get(0),
    )
}

fn format_tokens(n: f64) -> String {
    if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.1}k", n / 1_000.0)
    } else {
        format!("{:.0}", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suma_solo_asistente_desde_la_fecha() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"CREATE TABLE message (id TEXT, session_id TEXT, time_created INTEGER, time_updated INTEGER, data TEXT);
            INSERT INTO message VALUES ('1','s',100,0,'{"role":"assistant","tokens":{"total":1000,"input":900,"output":100}}');
            INSERT INTO message VALUES ('2','s',200,0,'{"role":"assistant","tokens":{"input":400,"output":100}}');
            INSERT INTO message VALUES ('3','s',200,0,'{"role":"user"}');
            INSERT INTO message VALUES ('4','s',50,0,'{"role":"assistant","tokens":{"total":9999}}');"#,
        )
        .unwrap();
        assert_eq!(tokens_since(&conn, 100).unwrap(), 1500.0);
        assert_eq!(format_tokens(1500.0), "1.5k");
        assert_eq!(format_tokens(2_300_000.0), "2.3M");
    }
}
