# AI Usage Widget

Widget flotante para Wayland que muestra cuánto te queda de cuota en **Codex** y **Claude Code**.

- **Aro**: límite de la sesión de 5 horas.
- **Barra**: límite semanal y cuánto falta para que se reinicie.

Solo aparecen las IAs que tengas instaladas (detecta `~/.codex` y `~/.claude`).

- **Bandeja**: clic en el icono para mostrar/ocultar; menú con *Configuración…* y *Salir*. El aro del icono muestra el límite más crítico (verde/amarillo/rojo).
- **Mover**: arrastra el widget; la posición se guarda en la config.

## Requisitos

- Un compositor Wayland con soporte de `wlr-layer-shell` (KDE Plasma, Hyprland, Sway…).
- Rust, GTK4 y gtk4-layer-shell. En NixOS, `shell.nix` ya los trae.
- Codex: haber usado `codex` al menos una vez (lee `~/.codex/sessions`).
- Claude: haber iniciado sesión con `claude auth login`.

## Uso

```bash
nix-shell
cargo run --release
```

## Providers extra

Desde *Configuración…* en el menú de la bandeja puedes agregar:

| Provider | Qué muestra | Necesita |
|---|---|---|
| **DeepSeek** | Saldo de la cuenta frente a un saldo de referencia (= 100%) | API key |
| **OpenRouter** | Crédito restante de la key frente a su límite | API key con límite |
| **OpenCode** | Tokens usados hoy frente a un presupuesto diario | Nada (lee `~/.local/share/opencode/opencode.db`) |

Las API keys se guardan en `config.json` con permisos `600`.

## Configuración

Se crea sola la primera vez en `~/.config/ai-usage-widget/config.json`:

```json
{
  "margin_top": 24,
  "margin_right": 24,
  "update_interval_secs": 60,
  "auto_refresh_claude": true,
  "providers": []
}
```

- `margin_top` / `margin_right`: separación respecto a la esquina superior derecha (se actualiza al arrastrar).
- `update_interval_secs`: cada cuántos segundos se actualiza.
- `auto_refresh_claude`: si el token de Claude está por vencer, lo renueva ejecutando `claude -p`.
