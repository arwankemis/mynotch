# Changelog

All notable changes and release notes for mynotch are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to Semantic Versioning.

---

## [0.0.1.4] - 2026-10-31

This release adds native Liquid Glass themes, a continuous spring animation coordinator, in-place streaming updates, and an in-app mod catalog.

### Added
- **Native Liquid Glass Materials:** Added `Glass Lite` and `Glass Adaptive` theme options alongside `Solid`. Uses native Desktop Window Manager (DWM) backdrop blur with custom polygon contour clipping for seamless window edges.
- **Continuous Spring Animation Engine:** Replaced fixed-step lerp with a sub-stepped semi-implicit Euler spring coordinator running at up to 240 Hz. Stage and tab changes retarget mid-flight without positional jumps.
- **Automatic Streaming In-Place Updater:** Self-update system that downloads releases from GitHub, validates SHA-256 signatures, performs zip-slip-safe extractions, and uses a health-checked helper process with automatic rollback on failure.
- **Mod & Extension Catalog:** Added a dedicated catalog under Settings to discover, install, toggle, and uninstall official extensions with a single click.
- **Daya Energy Monitor:** Added an optional hardware power monitoring extension with out-of-process sensor isolation.
- **Project Licensing:** Added root MIT license and updated third-party notices covering Piccolo and Zip libraries.

### Fixed
- **Player Glyphs Encoding:** Restored proper UTF-8 symbols for transport controls (Play, Pause, Skip, Previous) and volume buttons on Glass themes.
- **Midnight Calendar Rollover:** Fixed calendar state so dates and schedules advance automatically past midnight without requiring manual interaction.
- **Path Sanitization:** Removed hardcoded local developer paths, replacing them with dynamic environment variables and relative path lookups.
- **Compiler Hygiene:** Resolved all compiler warnings and dead code annotations across public and personal builds.
- **Driver Binaries Untracked:** Removed heavy binary driver installers and source archives from Git tracking to keep the repository lightweight.

### Security
- In-place updater network access is restricted strictly to official GitHub endpoints (`api.github.com`, `github.com`, `githubusercontent.com`).
- Public release binaries completely isolate internal audio daemon calls, preventing unneeded process spawning on user systems.

---

## [0.0.1.3] - 2026-10-29

This release adds community mod support with Lua scripting, Bluetooth audio/TWS battery tracking, and an integrated volume overlay.

### Added
- **Community Mod System:** Support for custom mods packaged as `.zip` archives or `.json` manifests extracted to `data/mods/`. Mod management in Settings includes install, toggle, uninstall, Tab visibility, and network permission controls.
- **Sandboxed Lua Engine:** Embedded Piccolo Lua 5.4 engine running with instruction limits and restricted system APIs (`ui.card`, `ui.button`, `ui.text`, `ui.headline`, `ui.ranked`, `system.toast`, `system.open_url`, `system.copy`, `system.config`, `system.json_decode`, and cached `system.http_get`).
- **Bluetooth & TWS Monitoring:** Detects connected Bluetooth audio devices via Windows Device Enumeration, shows connection toast notices, and displays battery levels on Stage 1 when reported by the device.
- **Custom Volume HUD:** Replaces the default Windows volume OSD while the notch is active, tracking media keys, laptop function keys, and notch wheel scrolls.
- **Sample Mods:** Added `ihsg-monitor`, `kurs_usd`, and `counter_lua` under `examples/mods/`.

### Fixed
- **Rank Lane Coloring:** Value styling in mod ranking lanes now respects individual signs rather than applying the lane theme uniformly.
- **Stage 1 Layout:** Placed TWS battery before laptop battery and expanded pill bounds dynamically to prevent truncated percentages.
- **Settings Versioning:** Removed hardcoded version strings from the General settings menu.

---

## [0.0.1.2] - 2026-10-28

This release adds mouse wheel volume control, laptop battery monitoring, Open-Meteo weather integration, clipboard pinning and search, and startup configuration.

### Added
- **Mouse Wheel Volume Control:** Scroll over the notch (Stage 0 and Stage 1) to adjust Windows master volume via Core Audio with an immediate HUD overlay.
- **Laptop Battery Indicator:** Shows current battery percentage, charging state, and low-battery alerts in the Stage 1 bar beside the clock.
- **Open-Meteo Weather Mod:** Native weather panel with temperature, condition summary, feels-like temperature, humidity, wind speed, rain probability, and hourly forecasts. Renders dynamic sky gradients and MGC weather atlas icons. Supports city presets (Jakarta, Bandung, Surabaya) and custom coordinates.
- **Clipboard Pinning & Search:** Pin frequently used clips to the top of the history list and filter entries via real-time search.
- **Windows Startup Option:** Toggle automatic startup on login via user registry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`) without background services.
- **Release Checker:** Check for new GitHub releases directly from Settings > Updates.

### Fixed
- **Stage 1 Layout Normalization:** Standardized 8 px spacing around dividers, added text ellipsis to protect clock and battery visibility, and smoothed compact morph transitions.
- **Temporary Media Status:** Song changes now surface in Stage 1 for 5 seconds before reverting to the active view.
- **Weather Icon Mapping:** Restored the cloud navigation icon on the weather tab.

---

## [0.0.1.1] - 2026-10-27

This release introduces Quick Sticky Notes, bilingual interface localization, modular mod support, and fixes across input and memory management.

### Added
- **Quick Sticky Notes:** In-notch notes tab with a two-column layout, five pastel color themes, auto-save, and direct export to Markdown or plain text. Includes a pop-out button (`⧉ Pop`) that detaches a note into an independent, always-on-top desktop window.
- **Bilingual Interface (ID / EN):** Complete English and Indonesian localization across tab headers, calendar views, reminder settings, prayer times, dialogs, and toast notices.
- **Modular Mod Structure:** Introduced manifest-based mod definitions (`.mod.json`) managed under Settings without background daemon overhead.
- **Context-Aware Stage 1:** Active music playback and running focus timers remain visible on Stage 1. Other tab updates display for 5 seconds before returning to the date and time view.
- **Interaction Enhancements:**
  - `Ctrl+A` select-all support in text inputs without triggering the Windows default error chime.
  - Mouse wheel scrolling to switch months in Calendar and navigate pages in Clipboard history.
  - "Follow" lyrics button appears when scrolling through lyrics manually to snap back to the active line.
  - Keyboard navigation with visible focus rings and `Enter`/`Space` activation across buttons and cards.
  - 140 ms hover debounce to prevent accidental expansions when sweeping past the notch.

### Fixed
- **Note Editor Isolation:** New notes no longer inherit text from previously deleted drafts.
- **DirectWrite Multiline Wrapping:** Text renderer correctly wraps long lines instead of clipping off-screen.
- **Memory & Resource Cleanup:** Fixed lingering GDI handles and ICU allocations in lyrics and clipboard handlers.
- **Clipboard Privacy Filter:** Respects password manager flags (`ClipboardHistoryIgnore` and `ExcludeClipboardContentFromMonitorProcessing`) to keep sensitive credentials out of clipboard history.
- **Menu Dismissal:** Settings and reminder menus close cleanly whenever the notch collapses or returns to idle.

---

## [0.0.1] - 2026-10-26

Initial alpha release of mynotch for Windows x64.

### Features
- **Three-Stage Notch Interface:** Built with Rust, Direct2D, and DirectWrite, adapting between compact bar and expanded panels.
- **Music & Lyrics:** System media transport controls and synchronized lyric display.
- **Calendar & Reminders:** Monthly calendar with Indonesian national holidays, daily reminders, and prayer time notifications.
- **Clipboard History:** Local, searchable history of copied text items.
- **Portable Architecture:** All configuration and state files remain relative to the application folder. Requires no installer, registry modifications, or administrative privileges.
