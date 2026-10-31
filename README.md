# mynotch

A lightweight, portable notch utility for Windows 10 and 11, built natively with Rust, Direct2D, and DirectWrite.

mynotch puts quick everyday tools right at the top of your screen. It runs directly against native Windows APIs with minimal resource usage, without web runtimes, Electron, or Chromium overhead.

---

## How It Works

mynotch rests at the edge of your primary display and adapts across three distinct stages:

- **Stage 0 (Collapsed):** A discreet pill along the screen edge that stays out of your way while working.
- **Stage 1 (Compact Bar):** Expands on hover to show the time, laptop battery, connected Bluetooth audio battery, and current playback title.
- **Stage 2 (Expanded Panel):** Opens on click to reveal full interactive panels for music, calendar, reminders, clipboard, notes, weather, and modular extensions.

Scrolling the mouse wheel over the notch bar in Stage 0 or Stage 1 adjusts your Windows master volume. The app handles volume changes with an unobtrusive on-screen HUD, temporarily suppressing the native Windows volume overlay while the notch is active.

---

## Key Features

### Themes & Visual Styles
Choose between three materials under **Settings -> General**:
- **Solid:** Crisp, opaque styling optimized for performance on any hardware.
- **Glass Lite:** Semi-translucent material with subtle background tinting.
- **Glass Adaptive:** Dynamic native DWM backdrop blur that transitions smoothly as the notch resizes.

### Music & Lyrics
- Integrates with Windows Media Transport Controls (Spotify, YouTube Music in browsers, Apple Music, and local media players).
- Transport buttons (Play, Pause, Skip, Previous) and interactive seek bar.
- Real-time audio spectrum visualizer.
- Timed synchronized lyrics with automatic CJK Romanization support.

### Bluetooth & Audio Devices
- Detects connected Bluetooth headphones, headsets, and speakers.
- Displays device battery levels directly in the Stage 1 bar when reported by Windows.
- Shows brief connection toasts when audio devices connect or disconnect.

### Calendar & Events
- Monthly view with mouse wheel navigation.
- Includes Indonesian national holidays and shared leave days.
- Automatically rolls over to the new date at midnight without requiring manual refresh.

### Daily Reminders
Four built-in reminders you can configure independently:
- **Focus Timer:** Pomodoro-style work and break intervals.
- **Hydration:** Regular drink reminders with daily intake targets.
- **Movement:** Periodic stretch reminders.
- **Prayer Times:** Accurate daily prayer schedule based on coordinates for Indonesian cities.

### Sticky Notes
- Quick desktop scratchpad with color-coded notes.
- Pop notes out into floating, borderless desktop windows.
- Export notes directly to Markdown (`.md`) or plain text (`.txt`).

### Clipboard History
- Tracks copied text locally.
- Search filter and pinning for frequently used snippets.
- Automatically ignores password managers and sensitive clipboard formats.

### Weather
- Powered by Open-Meteo with no API key required.
- Current temperature, conditions, humidity, wind speed, and rain probability.
- Dynamic gradient background reflecting daylight and precipitation.

### Sandboxed Extensions & Mods
- Install community extensions distributed as `.zip` or `.json` manifests.
- Runs on an isolated Lua 5.4 runtime (Piccolo) with strict instruction limits and opt-in network permissions.
- Example mods included in `examples/mods/` (currency converter, Lua counter, IHSG stock monitor).

### Automatic Updates
- Check for new releases directly from **Settings -> Updates**.
- Downloads signed GitHub release archives over HTTPS.
- Safe in-place file replacement with SHA-256 validation and automatic rollback if an update fails.
- *Note:* Upgrading from `v0.0.1.4` or earlier requires a one-time manual file replacement. In-app automatic updates are active starting from `v0.0.2` and later.

---

## Getting Started

mynotch is completely portable and requires no installer:

1. Download `mynotch-0.0.2-windows-x64.zip` (or `mynotch.exe` and `libunwind.dll`) from the [Releases](https://github.com/arwankemis/mynotch/releases) page.
2. Extract the files into a folder of your choice.
3. Run `mynotch.exe`.

All user settings, notes, and clipboard data are stored locally in the `data/` folder next to the executable. To back up or move your installation, simply copy that folder.

---

## Keyboard Shortcuts

| Shortcut | Description |
|---|---|
| `Alt + N` | Toggle notch open or closed |
| `Ctrl + A` | Select all text in input fields |
| `Esc` | Close open popup, modal, or return to previous tab |
| `Space` / `Enter` | Activate focused button |
| `Tab` / `Shift + Tab` | Navigate focus between controls |
| `Mouse Wheel` | Adjust volume on notch bar, change calendar months, or switch clipboard pages |

---

## System Requirements

- **Operating System:** Windows 10 (64-bit) or Windows 11.
- **Graphics:** Direct3D 11 and Direct2D support (automatically falls back to WARP software rendering if GPU acceleration is unavailable).

---

## Building from Source

To compile mynotch locally, you will need:
- Rust toolchain (`stable-x86_64-pc-windows-gnullvm` or `stable-x86_64-pc-windows-msvc`).
- PowerShell 5.1 or newer.
- (Optional) .NET 10 SDK if building the out-of-process hardware sensor provider.

Clone the repository and run the build script:

```powershell
git clone https://github.com/arwankemis/mynotch.git
cd mynotch
powershell -ExecutionPolicy Bypass -File scripts\build.ps1
```

The compiled binary will be placed in `dist/public/mynotch.exe` and copied to the project root.

To run the full unit test suite:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Test
```

---

## License

This project is licensed under the [MIT License](LICENSE). Third-party open-source components and licenses are documented in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
