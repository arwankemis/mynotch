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

### Added
- **Community Mod System:** Support for custom mods packaged as `.zip` archives or `.json` manifests. Includes isolated Lua 5.4 scripting environment (Piccolo) with instruction limits and opt-in network permissions.
- **Bluetooth & TWS Battery Monitoring:** Background polling for connected Bluetooth headsets and speakers, surfacing battery percentages in the Stage 1 bar.
- **Windows Volume HUD:** Intercepts and replaces the default Windows volume overlay with a notch-based HUD when active.
- **Sample Extensions:** Added sample mods in `examples/mods/`, including the IHSG market monitor and currency tracker.

### Fixed
- Rank lane coloring now respects individual value signs rather than the entire column color.
- Widened Stage 1 pill dimensions to accommodate long battery percentages.

---

## [0.0.1.2] - 2026-10-28

### Added
- Mouse wheel master volume control directly over the notch bar.
- Laptop battery level indicator on Stage 1.
- Open-Meteo weather mod with sky conditions, humidity, wind, and precipitation probability.
- Clipboard history search and item pinning.
- Option to launch mynotch on Windows startup via the user registry.

---

## [0.0.1.1] - 2026-10-27

### Added
- Sticky notes mod with pop-out desktop window mode and Markdown/plain text export.
- Bilingual Indonesian and English interface support with instant switching.
- Enhanced stage morphing animations and layout adjustments.

---

## [0.0.1] - 2026-10-26

- Initial alpha release of mynotch.
- Core three-stage notch bar built with Rust, Direct2D, and DirectWrite.
- Basic music playback controls, monthly calendar, and local clipboard storage.
