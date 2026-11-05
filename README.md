# AirPlay

Windows music player. The release build is a single exe.

Decoding uses Symphonia; output uses cpal in WASAPI shared mode.

UI is moving to [Ely GPUI Components](https://github.com/ELY-Open-Source/Ely-GPUI-Components) on [GPUI](https://www.gpui.rs). The current tree still has a legacy Win32 UI under `src/ui/`.

## Build

```bat
cargo build --release
```

Output: `target\release\AirPlay.exe`

## Features

- Borderless main window: drag, resize, minimize, maximize, fullscreen, always-on-top, mini mode
- Play / pause / stop / previous / next, seek, volume
- Formats Symphonia supports (mp3, flac, wav, ogg, aac, m4a, aiff, and more)
- Playlists in `playlists\*.m3u8`; config in `airplay.ini` next to the exe
- Multiple playlists, search, sort, group by folder, CUE sheets, drag-and-drop, network URLs (download then play)
- LRC lyrics, desktop lyrics, cover art, 10-band EQ, spectrum
- Global hotkeys, tray, taskbar progress, skins (`skins\` and `plugins\wallpaper\`)

WASAPI exclusive mode has no stable API here. The settings “low latency” option still uses shared mode.
