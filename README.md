# AI Usage Widget

Widget flotante para Wayland que muestra cuánto te queda de cuota en **Codex** y **Claude Code**.

- **Aro**: límite de la sesión de 5 horas.
- **Barra**: límite semanal y cuánto falta para que se reinicie.

Solo aparecen las IAs que tengas instaladas (detecta `~/.codex` y `~/.claude`).

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

## Configuración

Se crea sola la primera vez en `~/.config/ai-usage-widget/config.json`:

```json
{
  "margin_top": 24,
  "margin_right": 24,
  "update_interval_secs": 60,
  "auto_refresh_claude": true
}
```

- `margin_top` / `margin_right`: separación respecto a la esquina superior derecha.
- `update_interval_secs`: cada cuántos segundos se actualiza.
- `auto_refresh_claude`: si el token de Claude está por vencer, lo renueva ejecutando `claude -p`.
