# AirPlay — Agent Guide

Windows-only music player (single `AirPlay.exe`). Decode via Symphonia; audio out via cpal WASAPI shared mode.

## UI stack (required)

**Going forward, all UI work uses [Ely GPUI Components](https://github.com/ELY-Open-Source/Ely-GPUI-Components)** (`ely-gpui-component`) on [GPUI](https://www.gpui.rs). Do not extend the legacy Win32/`windows` crate UI for new features; migrate or replace `src/ui/` toward Ely instead.

Docs / live gallery: https://elygpui.com

Typical deps (pin versions to what Ely documents; adjust when upgrading Ely):

```toml
ely-gpui-component = { git = "https://github.com/ELY-Open-Source/Ely-GPUI-Components" }
gpui = { package = "gpui-pre", version = "=0.3.11" }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.11", features = ["font-kit"] }
```

On Windows, prefer `font-kit` only unless a dependency forces Linux window backends. Call `ely_gpui_component::init(cx)`, register `Assets`, and set theme via `Theme::set_mode` before opening windows. Prefer Ely widgets/themes over ad-hoc custom controls when an Ely component exists.

## Commands

```bat
cargo build
cargo build --release
cargo check
cargo test
```

Release binary: `target\release\AirPlay.exe`

## Layout

| Path | Role |
|------|------|
| `src/main.rs` | Entry; panic → exit code |
| `src/app.rs` | App state / orchestration |
| `src/player/` | Decode, EQ, FFT, playback |
| `src/ui/` | **Legacy** Win32 UI — replace with Ely/GPUI; do not grow this path for new UI |
| `src/playlist.rs` / `track.rs` / `cue.rs` / `lrc.rs` | Playlist, tracks, CUE, lyrics |
| `src/config.rs` / `skin.rs` | `airplay.ini`, skins |
| `src/meta.rs` / `net.rs` / `textutil.rs` | Tags, HTTP download, text helpers |

Config and playlists live beside the exe at runtime (`airplay.ini`, `playlists\`, `skins\`). Do not assume they exist in-repo.

## Conventions

- Ship target remains Windows single-exe; keep playback on cpal/WASAPI.
- New UI: Ely GPUI Components + GPUI only — no new Win32 GDI UI, no webview shell.
- Prefer small, focused changes that match nearby module style.
- Keep release as one exe; avoid new runtime-required sidecar binaries unless necessary.
- WASAPI exclusive / true low-latency has no stable public API here — do not claim or wire exclusive mode without a clear design.

## Git commits

Use [Conventional Commits](https://www.conventionalcommits.org/) in **English**.

### Format

```
type: short summary
type(scope): short summary
```

Optional body after a blank line. Subject line only is fine for most changes.

### Types

| Type | Use when |
|------|----------|
| `feat` | New user-facing capability |
| `fix` | Bug fix |
| `refactor` | Internal change, same behavior |
| `perf` | Performance improvement |
| `docs` | Docs only (README, AGENTS, comments-as-docs) |
| `style` | Formatting / naming with no logic change |
| `test` | Tests only |
| `build` | Cargo / build script / dependency plumbing |
| `chore` | Misc maintenance that is not the above |

### Rules

- Subject in English, imperative mood (`add`, `fix`, `remove` — not `added` / `fixes`).
- Lowercase the summary after the colon (`feat: add tray mute` not `feat: Add tray mute`).
- **No trailing period** on the subject line.
- Keep the subject short (aim ≤ 72 characters).
- Scope is optional; use a short module name when helpful (`feat(ui): …`, `fix(player): …`).

### Examples

```
feat: add CUE sheet multi-track import
feat(ui): support mini-mode window snap
fix: restore volume after device change
fix(playlist): handle empty m3u8 without panic
refactor: split EQ processing out of decode loop
perf: reuse FFT buffer across spectrum frames
docs: document release build output path
build: bump lofty to 0.22
chore: ignore local skin scratch files
```

### Bad

```
feat: add tray mute.
Added tray mute support
update stuff
fix: 修复音量问题
```
