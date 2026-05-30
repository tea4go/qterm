# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build                      # debug build
cargo build --release            # release build (LTO + strip)
cargo check                      # quick type-check
cargo run                        # build + run debug
```

PowerShell wrapper (`build.ps1`):
```powershell
.\build.ps1                      # debug build + run
.\build.ps1 -Release             # release build + run
.\build.ps1 -BuildOnly           # build only, don't run
.\build.ps1 -Clean               # cargo clean then build + run
```

No test suite exists yet. Use `cargo check` for fast verification.

## Architecture

QTerm is a GPU-accelerated terminal emulator built with Rust + egui/eframe. The UI has a custom title bar (no native decorations), a left sidebar with ribbon + connection list, and a central terminal area supporting split panes.

### Data flow

```
main.rs → QTermApp (app.rs)
  ├── Tab (tab/tab_item.rs) — owns a SplitLayout
  │     └── SplitLayout (ui/split_pane.rs) — manages ChildPanes (max 6)
  │           └── ChildPane::PaneKind
  │                 ├── Terminal { terminal, backend: PtyHandle | SshHandle }
  │                 └── Sftp { panel }
  ├── AppConfig (config.rs) — window state, saved to APPDATA/qterm/config.ini
  ├── Preferences (config.rs) — fonts/theme from APPDATA/WhaleTerm/preferences.json
  └── AppTheme (theme/) — SystemTheme + TerminalTheme + ExtraTheme
```

### Terminal pipeline

1. **PTY/SSH** → raw bytes via channel (`reader_rx`)
2. **Terminal::feed()** → `vte::Parser` → updates `Grid` (cells with char + colors + attrs)
3. **renderer::render()** → reads `Grid`, draws with egui `Painter` using `TerminalTheme` colors
4. **User input** → keyboard/mouse events → writes bytes back to PTY/SSH

### Key modules

- **`terminal/`** — Grid (scrollback buffer), Cell (char + ANSI attrs), Parser (vte escape sequences), Renderer (egui painting)
- **`theme/`** — `SystemTheme` (UI colors, applies to egui Style), `TerminalTheme` (ANSI 16/256 colors, cursor), `ExtraTheme` (SFTP progress, tables). All colors are hardcoded hex with `parse_color()`.
- **`ssh/`** — `SshHandle` wraps russh with tokio runtime, `SshClient` is the russh Handler
- **`sftp/`** — `SftpHandle` wraps russh-sftp, opened from an existing SSH connection
- **`pty/`** — `PtyHandle` wraps portable-pty for local shell sessions
- **`connection/`** — Reads WhaleTerm's `connections.json`, decrypts AES-256-CFB passwords (key derived from motherboard serial)

### Configuration sources

| File | Location | Purpose |
|------|----------|---------|
| `config.ini` | `APPDATA/qterm/` | Window position/size, font zoom level |
| `preferences.json` | `APPDATA/WhaleTerm/` | Font families/sizes/bold per section, theme |
| `connections.json` | `APPDATA/WhaleTerm/` | SSH connections with encrypted passwords |

Font config mapping from `preferences.json`:
- `config.defaultFontFamily/Size/Bold` → main body default font
- `general.fontFamily/Size/Bold` → left sidebar/outline font
- `shell.fontFamily/Size/Bold` → terminal and SFTP font

### UI layout (rendered in `app.rs` update loop)

```
┌─ Title Bar (40px, custom drag area + window controls) ────────┐
│ [QTerm] [Tab1] [Tab2] [+]                        [-][□][x]   │
├────┬─────────────┬────────────────────────────────────────────┤
│ >_ │ Connections │ Terminal / SFTP panes                      │
│  F │  Group 1    │                                            │
│    │   host1     │  (split up to 6 panes, H or V)            │
│    │   host2     │                                            │
│    │ Open Tabs   │                                            │
│    │  tab1       │                                            │
│    │             │                                            │
│ [L]│             │                                            │
├────┴─────────────┴────────────────────────────────────────────┤
│ ● session | connected    Ctrl+T New | Ctrl+Shift+N SSH ...   │
└───────────────────────────────────────────────────────────────┘
```

### Keyboard shortcuts

- `Ctrl+T` / `Ctrl+W` — new/close tab
- `Ctrl+Shift+H/V` — split pane horizontal/vertical
- `Ctrl+Shift+W` — close active pane
- `Ctrl+Arrow` — cycle panes
- `Ctrl+B` — toggle left sidebar
- `Ctrl+Shift+N` — SSH dialog
- `Ctrl+Shift+F` — open SFTP from active SSH pane
- `Ctrl+/-` — font zoom in/out
- `L/D` button in ribbon — toggle light/dark theme
