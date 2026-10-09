<div align="center">
  <img src="src-tauri/icons/icon.png" alt="HotBoard logo" width="128" />
  <h1>HotBoard</h1>
  <p><strong>A lightweight offline hotkey soundboard for Windows.</strong></p>
</div>

Import an MP3 or WAV, press a shortcut, and play it from anywhere — Discord, a game, your
browser. HotBoard stays in the system tray and keeps listening.

|  |  |
| --- | --- |
| **Installer size** | 1.73 MiB |
| **Idle memory** | 29.2 MiB |
| **Network** | Never, after install |
| **Platform** | Windows 10 / 11 (x64) |

---

## Features

- **Global shortcuts** that work while other applications are focused or minimized
- **Tray resident** — close the window and it keeps playing from the tray
- **Shortcut validation** rejects combinations Windows reserves (`Ctrl+C`, `Alt+Tab`, `Win+L`)
  and warns before you bind a bare key that would swallow typing everywhere
- **Honest failure** — if Windows refuses a shortcut, the app says so instead of pretending it works
- **Local only** — no account, no telemetry, no cloud, no database, no server
- **Non-destructive** — imported audio is copied into app storage; your originals are never touched

## Requirements

- Windows 10 (1803 or newer) or Windows 11, 64-bit
- [Rust](https://rustup.rs/) and the [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

## Getting started

```bash
npm install
npm run tauri dev
```

> **Windows and Rust not on your PATH?**
> If you see `program not found` for `cargo`, add Rust once:
> ```powershell
> setx PATH "%PATH%;%USERPROFILE%\.cargo\bin"
> ```
> Then open a new terminal.

> **First launch is slow.** Tauri compiles the full Rust dependency tree on the first
> `dev` run — expect several minutes. Later runs start in seconds.

## Building an installer

```bash
npm run tauri build
```

The installer is written to:

```
src-tauri/target/release/bundle/nsis/HotBoard_<version>_x64-setup.exe
```

It installs to `%LOCALAPPDATA%\Programs\HotBoard` and adds Start menu and desktop
shortcuts. Uninstall through **Settings → Apps → Installed apps**.

### Distributing it

Two things to know before you share the file:

- **It is unsigned.** Windows SmartScreen shows a blue warning on first run. Users click
  **More info → Run anyway**. On managed corporate machines, policy may block this outright.
- **WebView2 is not bundled.** Windows 10 (1803+) and 11 ship WebView2 already, so most
  users never notice. On a stripped-down machine, the installer downloads it on first run —
  which needs a connection.

To make the installer fully offline, set this in `src-tauri/tauri.conf.json`:

```json
"windows": { "webviewInstallMode": { "type": "offlineInstaller" } }
```

That embeds WebView2 and grows the installer from 1.73 MiB to roughly 130 MiB.

## How it works

A Tauri 2 desktop app: React and TypeScript for the interface, Rust for everything that
touches the system.

```
src/            React UI — sound list, editor, shortcut recorder
src-tauri/src/
  lib.rs        App state, sound CRUD, shortcut registration, lifecycle
  shortcut.rs   Key mapping, reserved-combination checks, validation
  audio.rs      rodio playback and decode verification
  store.rs      library.json + managed audio files on disk
  tray.rs       System tray icon, Open and Quit
```

Shortcuts are handled natively by `tauri-plugin-global-shortcut`, so they fire while other
applications have focus. Audio plays in-process through `rodio` — no external player process
is ever launched.

## Where your data lives

```
%APPDATA%\com.hotkeysoundboard.app\
  library.json    sound names, shortcuts, settings
  sounds\         copies of the audio you imported
```

Deleting that folder resets HotBoard to a clean install. The app identifier determines this
path, so changing it in `tauri.conf.json` orphans existing data.

## Scripts

| Command | Purpose |
| --- | --- |
| `npm run tauri dev` | Run the app with hot reload |
| `npm run tauri build` | Produce the Windows installer |
| `npm run build` | Type-check and build the frontend only |
| `npm run test` | Run the Rust test suite |

## Troubleshooting

**"This audio file could not be decoded"**
The file's extension is misleading. HotBoard accepts MP3 and WAV; verify the file really is
one of those and is not truncated or corrupt.

**A shortcut doesn't fire**
Another program is holding that combination. HotBoard reports this on startup — check the
warning banner listing which shortcuts Windows refused.

**`localhost refused to connect` on launch**
The dev server bound to IPv6 only. `vite.config.ts` pins it to `127.0.0.1`; if you changed
that, put it back.

## License

Private project. All rights reserved.