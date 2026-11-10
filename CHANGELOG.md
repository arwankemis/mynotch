# Changelog

All notable changes and release notes for mynotch are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to Semantic Versioning.

---

## [0.0.2.6] - 2026-11-10

This release introduces the startup laser edge sweep, interactive Stage 2 card spotlights, footer tab pagination indicators, fixes Stage 1 lyric collision against privacy badges, restores full ears rendering on Stage 2 lyric mode, optimizes player layout spacing, removes the heavy Power companion background daemon from the public edition for minimal memory usage, fixes settings BOM save locks, resolves sticky note inputs floating over overlays, and unifies notch ear contour rendering in Weather and Quick Controls.

### Added
- **Startup clean laser edge sweep:** notch startup now plays a smooth one-shot laser edge beam sweeping from left to right along the physical contour (ears and bottom edge) following the fade-in drop animation; the top edge along the monitor bezel remains dark and seamless.
- **Stage 2 spotlight hover cards:** cards in Stage 2 (Quick Controls, Weather forecast, and Mod manifests) now render interactive pointer-tracking radial spotlights and accent border highlights within a 140 DIP radius.
- **Stage 2 footer tab indicator:** a clean monochrome pagination indicator in the center footer shows available tabs with active rounded capsules (`#f4f4f6`) and inactive transparent dots (`rgba(255,255,255,0.25)`), supporting direct click navigation.
- **Open geometry notch rim path:** introduced `rim_path` in `render.rs` to isolate the lower notch rim (ears, walls, bottom corners, and bottom edge) from the closed top edge ($y = 0$).
- **Stage 1 dual-line Japanese & Romaji lyrics:** Stage 1 lyric stretch view now supports dual-line display for Japanese songs, showing original Kanji/Kana text alongside Romaji transliteration. Implemented non-destructive dual-line parsing (`LyricLine::romanized`), extended Japanese phonetic compound dictionary, and adaptive multi-line layout planning.
- **Stage 1 lyric album thumbnail & chubby visualizer:** Stage 1 lyric stretch view now features a 20x20 rounded album cover thumbnail on the left slot (falling back to a music icon) and 4 chubby pill-shaped audio visualizer bars on the right slot that animate symmetrically from the vertical center following real-time audio levels.

### Changed
- **Stage 1 lyric typography hierarchy:** Optimized text hierarchy in dual-line lyrics to focus on Romaji (12.5 DIP, bold, `#ffffff`) while keeping original Japanese text as secondary context (10.5 DIP, regular weight, muted `#9499a4`).
- **Stage 1 now-playing layout & DirectWrite measurement:** Replaced character-count width estimation in Stage 1 now-playing status with cached DirectWrite layout measurement (`refresh_now_playing_metrics`). Anchored media brand icons firmly to the left bezel and aligned `[Title · Artist | Time]` inline to eliminate awkward whitespace gaps before the clock separator.
- **Stage 2 player height and spacing:** expanded Stage 2 music player height from 180 to 192 DIP. Repositioned album artwork, track title, and media source icon down to `y = 48` with a generous 12 DIP clearance from the header tabs. Aligned scrubber (`y = 114`) and transport controls (`y = 135`) to guarantee a balanced 12 DIP breathing room above the footer tab indicator.
- **Stage 1 hardware monitor as passive indicator:** When enabled in Settings, the Stage 1 CPU/RAM/GPU monitor pill functions as a lightweight passive display indicator; clicking it no longer triggers non-existent expansion routes.
- **Window canvas expansion:** expanded `CANVAS_WIDTH` from 620 DIP to 660 DIP so the 620 DIP Stage 2 Lyric Mode panel retains 20 DIP of clearance on each side, preventing its 12 DIP ear fillets from being clipped by the window boundary.
- **Stage 1 now-playing width formula:** refined `now_playing_target_geometry` to accurately size the Stage 1 pill between 230 and 480 DIP, preventing song titles from being prematurely truncated ("Sway · CO...").

### Removed
- **Power/Daya tab & companion background daemon (Public edition):** Completely removed the standalone "Daya" (Power) tab and deactivated the automatic spawning of the `mynotch-power.exe` background helper daemon.
  - *Rationale & Memory Optimization:* Running a separate background process (`mynotch-power.exe`) alongside the main notch shell incurred significant memory and CPU overhead from heavy kernel sensor polling and driver services. For daily desktop use, Stage 1 native CPU, RAM, and GPU indicators already provide comprehensive hardware monitoring via lightweight built-in Windows performance counters with negligible memory footprint and without requiring an extra background companion process.
  - *Clean Mod Catalog & Automatic Migration:* Removed Daya from the built-in Mod Catalog and introduced automatic normalization in `Settings::normalize()` to purge lingering Daya mod entries from user settings and tab orders cleanly.

### Fixed
- **Settings Store save-lock on UTF-8 BOM:** Fixed persistent "Data gagal dibaca" and "Belum tersimpan — coba simpan lagi" notifications caused when settings files were written with a UTF-8 Byte Order Mark (`0xEF, 0xBB, 0xBF`). Added automatic BOM stripping in `Store::load()` so JSON deserialization succeeds reliably without locking writes.
- **Floating sticky note input controls on overlays:** Fixed a Win32 child window lifecycle bug where Catatan (Saved) `EDIT` controls remained visible and floating over the Quick Controls and Privacy detail overlays. Handlers now cleanly destroy and rebuild child inputs when transitioning to/from full-surface overlay views.
- **Disjointed notch ears and spotlight clipping in Weather & Quick Controls:** Fixed an issue where the top-left ear fillet appeared cut off and had a mismatched color when hovering the mouse in Weather and Quick Controls.
  - Removed redundant inner `p.fill` rounded rectangle (radius 18 DIP) in Quick Controls that produced an inward curved bite at the top corners and erased the pointer spotlight on the pill body while leaving it stranded on the ear.
  - Upgraded weather rendering to `weather_background_notch`, filling the unified Direct2D notch contour geometry (`contour_path`) across both ears and body in the base shell pass, eliminating top-left corner bites and ensuring seamless, continuous cursor-following spotlight illumination across the entire notch.
- **Stage 1 lyric mode privacy badge collision:** resolved a bug where `cursor_right` was not decremented when privacy indicators were active, causing centered lyric text to overlap the microphone badge. Added adaptive `lyric_wrap_limit` and dynamic `privacy_w + 14.0 DIP` pill width expansion.
- **Stage 2 lyric mode missing ears:** fixed clipping of the left and right ear fillets in the 620 DIP wide lyric view by widening `CANVAS_WIDTH` to 660 DIP.
- **Stage 2 mouse hover spotlight fill:** decoupled interior background spotlight fill (`FillGeometry(&contour_path)`) from border glow stroke (`DrawGeometry(&rim_path)`), restoring smooth cursor-following glow inside Stage 2 while keeping the top bezel border dark.
- **Spotlight test assertion:** corrected Euclidean distance calculation in `test_pointer_in_spotlight_range`.

---

## [0.0.2.5] - 2026-11-09

This release connects Quick Controls to real Windows radio and brightness hardware APIs, fixes the brightness slider drag and commit cycle, introduces the macOS-style Control Center icon, removes the overlapping Stage 2 mini visualizer, renders solid battery charging bolts on Stage 1, and improves YouTube session detection.

### Added
- **Windows radio integration:** Wi-Fi and Bluetooth cards in Quick Controls now discover real system adapters and toggle their radio state via Windows Runtime APIs (windows::Devices::Radios).
- **Physical display brightness control:** Display brightness read and write now connect directly to Windows WMI WmiMonitorBrightness and WmiMonitorBrightnessMethods.WmiSetBrightness.
- **Control Center icon:** Added a clean 2-slider Control Center outline icon (control_center) in render.rs, replacing the dense 3-line slider icon.
- **Stage 1 battery charging bolt:** Render a solid lightning bolt polygon instead of slanted triangles when the battery is charging.

### Changed
- **Stage 2 header layout:** Removed the redundant mini visualizer bars from the Stage 2 header that overlapped the Quick Controls button, giving the new Control Center button a dedicated, clean position.
- **YouTube detection:** Dynamically re-evaluate YouTube session detection on hover, tab switch, and settings toggle to keep CC and branding accurate.

### Fixed
- **Brightness slider snapping back:** Fixed WMI WmiSetBrightness parameter Timeout type mismatch (0x80041005 WBEM_E_TYPE_MISMATCH) by passing signed VT_I4 instead of unsigned VT_UI4.
- **Brightness slider drag & commit:** Slider now updates preview smoothly on WM_MOUSEMOVE and properly commits to hardware on mouse release (WM_LBUTTONUP). Local state updates immediately upon successful commit to eliminate slider bounce-back.
- **Keyboard navigation:** Prevented Space and Enter keys from mistakenly triggering QuickBrightnessPointer.
- **Quick Controls focus reset:** Reset ocused_element when exiting Quick Controls so reopening always starts with clean navigation state.
- **WMI memory management:** Properly parse SAFEARRAY brightness levels and release temporary COM VARIANT objects with VariantClear.

---
## [0.0.2.4] - 2026-11-08

This release adds the approved startup sequence and Stage 1 reminder accents, a Spotify-style Stage 2 music transport with a two-column lyric mode, mirrored now-playing bars, persistent reminder interval options, and makes the Saved (Simpan) tab a permanent built-in tab.

### Added
- **Stage 1 startup sequence:** the notch now boots into the tiny Stage 0 notch (fade + drop) and springs into the idle clock after 2.5 seconds, exactly like the approved mockup; hover still skips straight to Stage 1.
- **Stage 1 reminder accents:** reminder pills get a per-kind accent color (hydration cyan, prayer violet, movement green, focus amber) with an icon chip and progress ring, plus a soft accent gradient on the pill background.
- **Accent gradient sweep:** in the "Glow" accent mode a light band sweeps across the reminder pill every 3 seconds with the mockup easing; "Soft" keeps a static gradient, "Neutral" keeps the classic look.
- **Mirrored now-playing bars:** the lyric stretch view on Stage 1 shows a 4-bar waveform centered on its middle axis (matching the mockup) instead of bottom-aligned bars.
- **Persistent reminder intervals:** the Focus, Hydration, and Movement reminder menus now store the chosen interval in settings again, so a picked value survives restarts.

### Changed
- **Spotify-style transport:** the Stage 2 player uses the approved transport row (shuffle/previous/play/next/repeat, 4 px active dot, hover without background) driven by real Windows SMTC state, with a dedicated lyrics button.
- **Two-column lyric mode:** the lyric view widens the panel and splits lyrics into 40/60 columns with a smooth sheet-slide animation.
- **Saved tab is permanent:** the Notes/Simpan tab is now a built-in tab (no longer installed through the mod catalog), so a fresh install starts with Musik, Kalender, Reminder, and Simpan. Existing notes data is untouched, and the old `sticky_notes` package entry is migrated automatically.
- **Reminder quick tests removed:** the "Held reminders" backlog, the prayer test button, and the reminder test mode toggle are removed; reminders held during Quiet mode now simply reappear one by one (still silent) after Quiet is turned off.

### Fixed
- **Quick Controls honesty:** the panel is now labeled as a preview and no longer presents fictitious Wi-Fi/Bluetooth adapters or mock brightness values as real device state. Missing radios are reported as not detected, and unsupported displays show `--%` instead of a fabricated percentage.
- **Updater health signal:** the updated build reports itself healthy only after five seconds of stable rendering, not immediately after the first frame. The helper timeout is 45 seconds to cover startup plus the stability window.
- **Staging integrity:** a failure to hash a staged file now aborts staging instead of silently skipping verification for that file.
- **Transaction journal paths:** `install_dir`, `stage_dir`, `backup_dir`, and `target_exe` are validated against the transaction and installation roots, not just each managed file's relative path.
- **Icons:** added `close`, `lock`, and `headphones` outline icons so the reminder Dismiss button, Quiet indicator, and TWS battery indicator no longer fall back to a package icon.
- **Pop-out notes:** a pop-out window now closes even when capturing its editor fails, instead of refusing to close indefinitely.
- **New tasks:** `AddTodo` persists immediately, matching task deletion.

### Documentation
- Refreshed `SHA256SUMS.txt` to the 0.0.2.3 release digests; it previously held 0.0.2-era hashes.

---

## [0.0.2.3] - 2026-11-06

This release stabilizes Stage 1 routing, adds the Stage 1 hardware monitor and weather preview, merges Clipboard and Notes into one Saved tab, and removes the custom theme system in favor of the three built-in looks.

### Added
- **Stage 1 hardware monitor:** CPU, RAM, and GPU metrics selectable individually, drawn as stable-width concentric gauges with per-metric colors and high/critical thresholds. Clicking the pill opens Stage 2.
- **Stage 1 weather preview:** leaving the Weather tab keeps a weather pill on Stage 1 for about three seconds with a sky-matched gradient; clicking it reopens the Weather tab.
- **Saved tab:** Clipboard, Notes, and Tasks now share one tab with sub-tabs, following the active language.
- **Task deletion:** a Delete/Hapus button removes the selected task, persists the change, and never deletes the linked note.
- **Export menu:** the note download icon opens a menu with `.txt` and Sticky Note (Markdown) exports.

### Changed
- **Stage 1 routing:** the calendar is the default idle state; track changes show a transient title/artist pill for about five seconds. Lyrics only appear after the user enables lyric mode from the Music tab.
- **Media detection:** Auto now falls back to a session that is actually playing when Windows reports no current session.
- **Stable layout:** fixed-width hardware value boxes, centered metrics, and a clamped Stage 1 layout prevent date trimming and pill resizing.
- **Neutral UI:** tab, accent, slider, and control colors are neutral across Settings and all built-in themes.

### Fixed
- Lyric wrapping for long lines no longer stalls and 1-line/2-line transitions no longer flicker.
- Hovering note editor fields no longer collapses Stage 2; the panel closes only after the pointer leaves the notch.
- Empty-note timestamp no longer overlaps the empty-state hint.
- Clipboard/Notes text and task actions stay consistent with the selected language.
- **Reminder action buttons:** Stage 1 activity pills drew the literal icon names ("cl…", "ri…", "ch…") instead of glyphs. Close, open, and confirm now render proper outline icons for Focus, Stretch, hydration, prayer, and task-due reminders.
- **Clipboard glyphs:** the clear (broom) and pin buttons now render icons instead of clipped text.

### Security
- **YouTube lyrics fallback:** the previous "not Spotify/VLC" gate was replaced by positive session evidence plus double consent (off by default). Candidate videos are verified by duration, title, and artist, and caption URLs are restricted to YouTube/Google Video hosts.
- **Update recovery:** a failed rollback now surfaces a `RollbackFailed` phase and manual-reinstall message instead of being reported as success. Staging hashes are verified before the swap, `RestartNow` keeps the pending record, and the provider downloader refuses packages without a SHA-256 digest.

### Removed
- **Custom themes:** Cyberpunk Neon, OLED Pure Black, and Retro Classic examples and the theme loader are removed. Only Solid, Glass Lite, and Glass Adaptive remain, plus the light/dark switch.
- Separate `.md`/`.txt`/PopOut export buttons in the Notes header.

---

## [0.0.2.2] - 2026-11-01

This release delivers borderless fullscreen gaming focus protections, YouTube timedtext caption fallback for synchronized lyrics with intelligent line wrapping, extended hardware telemetry (CPU, GPU, RAM, multi-drive storage) in the Power mod, custom media player iconography, and bilingual UI refinements.

### Added
- **YouTube Captions Fallback:** Added synchronized lyrics fetching from YouTube timedtext captions when songs are not indexed on LRCLIB. Includes a browser window heuristic to ensure captions only query when YouTube media playback is active.
- **Lyric Line Splitting (`split_long_lines`):** Lyric lines exceeding 40 characters are automatically split across word boundaries with smooth, interpolated timestamps to prevent text truncation on compact notch displays.
- **RAM & Hardware Load Telemetry:** Extended system telemetry to monitor RAM utilization (used/total GB and percentage) via `GlobalMemoryStatusEx`, CPU total load, and GPU load with responsive sub-metric progress bars in the Power mod.
- **Interactive Multi-Drive Storage Dropdown:** Added an interactive drive selector overlay displaying per-partition storage usage (`A:`-`Z:`) with storage progress bars, viewport clamping, and non-blocking asynchronous disk sampling guarded against offline media errors.
- **Dedicated Media Icons:** Added native vector icons for VLC Media Player and YouTube / YouTube Music in Stage 1 playback pills alongside Spotify.
- **Rounded Artwork Rendering (`cover_rounded`):** Render album art with smooth rounded geometric corner clipping via Direct2D layer masks.
- **Bilingual Interface Localization (EN / ID):** Full English and Indonesian localization across mod catalog titles ("Power" / "Daya"), tab headers, hardware sensor telemetry notices, and gauge units.

### Fixed
- **Borderless Gaming Focus & Fullscreen Detection (`WS_EX_NOACTIVATE`):**
  - Added `WS_EX_NOACTIVATE` window style so the notch never intercepts keyboard or mouse control from active games.
  - Rewrote borderless fullscreen detection (`is_borderless_fullscreen_rect`) to identify borderless windowed games, anti-DirectFlip margins (e.g. 1-2px variations), and custom resolution viewports while keeping normal maximized desktop application windows accessible.
- **Multi-Core CPU Load Spike:** Updated the companion power provider (`Program.cs`) to prioritize `CPU Total` load sensors over single-core spikes (`CPU Core #1`).
- **UI Thread Freezing on Storage Queries:** Offloaded multi-drive filesystem inspection to an asynchronous background worker (`self.job("Storage", ...)`), eliminating frame drops on slow or spinning hard drives.
- **Stage 1 Power Tab Persistence:** Excluded the Power / Daya tab from the 5-second calendar auto-revert timeout, allowing continuous hardware monitoring on Stage 1.
- **Weather Refresh Timestamp:** Displays the actual timestamp when the forecast was fetched (`weather_fetched_time`) rather than the meteorological slot time.
- **Redundant Back Button Removal:** Removed the duplicate in-page back button on the Privacy Indicator settings sub-page, consolidating navigation to the header back button and adjusting scroll height boundaries.

---

## [0.0.2.1] - 2026-10-31

This release resolves weather refresh caching and multi-monitor alignment, reinforces in-app auto-updater transactions, and stabilizes Direct2D WinRT threading apartments.

### Added
- **Live Weather Manual Refresh:** Manual refresh, coordinates update, and city preset selection force an immediate live forecast fetch from Open-Meteo, bypassing the disk cache with visual loading indicators.
- **Stage 1 Weather Auto-Revert:** Switching to Weather displays the forecast on Stage 1 for 5 seconds before returning to the calendar view.

### Fixed
- **In-App Updater Verification:** Enforced SHA-256 digest validation before staging and extraction, hardened crash-safe helper swaps with automatic transaction rollbacks, and strengthened archive directory traversal protections.
- **Multi-Monitor & DPI Alignment:** Corrected screen boundary calculations and notch positioning across secondary monitors and mixed DPI setups.
- **Direct2D / WinRT Apartment Stabilization:** Linked threading apartment initialization directly for robust Direct2D and WASAPI startup across Windows toolchains.
- **Reminder Cleanup:** Removed obsolete phantom eye reminder timer.

---

## [0.0.2] - 2026-10-31

This release delivers security hardening, network sandboxing for community extensions, clipboard privacy improvements, UI responsiveness enhancements, and reliability safeguards.

> [!NOTE]
> **Upgrade Notice:** Upgrading directly from `v0.0.1.4` or earlier via the in-app update button is not supported due to missing checksum discovery in older binaries. A manual update is required once by downloading `mynotch-0.0.2-windows-x64.zip` and replacing `mynotch.exe`. In-app automatic updates are fully supported starting from `v0.0.2` and later.

### Added
- Public distribution checks now inspect source, binaries, and nested ZIP contents before publication.
- **Clipboard Retention Policy:** Added automated expiration for clipboard history (24 hours for regular clips, 30 days for pinned clips) with a 64 KiB per-entry size limit. Added local storage transparency notice in empty clipboard view. Existing entries migrate cleanly on first run.
- **Configurable Network Allowlist:** Added `net_allowed_hosts` in `settings.json` to allow explicit configuration of allowed private/internal endpoints when needed.
- **Corrupt State Recovery Hold:** Safeguards corrupt configuration or state files against accidental overwrites, preserving existing data in a recovery snapshot and alerting the user on startup.
- **Atomic File Operations:** Sticky note text and markdown exports now write through temporary staging files before renaming, eliminating partial file writes on abrupt crashes.
- **Monitor Work Area Clamping:** Floating sticky note windows are now dynamically clamped within the visible work area of the active monitor.

### Fixed
- **In-App Updater SHA-256 Resolution:** Added automatic extraction and verification of SHA-256 checksums from GitHub release digests and release notes in the updater candidate selector.
- **Volume OSD UI Thread Freeze:** Offloaded initial Windows volume overlay detection to a background worker thread (`mynotch-osd`), preventing startup frame stutters.
- **Fullscreen Shell Detection:** Added bounding dimensions to prevent fullscreen shell surfaces on Windows 10 from accidentally being treated as volume overlays.
- **Painting State Guard:** Added RAII `PaintingGuard` ensuring rendering state flags are always reset even if drawing operations encounter unexpected conditions.
- **WinRT/COM Lifecycle Symmetries:** Paired WinRT initialization with RAII uninitialization in Bluetooth device query routines and extended audio/visualizer thread shutdown grace periods to 2 seconds.
- **ICU Parser ABI Alignment:** Fixed `UParseError` struct layout (72 bytes) and enforced ICU version gating (>= 50) in lyrics parser routines to prevent memory overruns.
- **Narrow Notch Reminder Notice:** Replaced empty reminder view with bilingual guidance text on compact notches (<440 px).
- **Lyrics Connectivity Feedback:** Differentiated network fetch errors from genuine "no lyrics found" track states.
- **Codebase Dead Code Cleanup:** Removed superseded legacy module `shadow.rs`.

### Security
- **SSRF & Private Network Guard:** Transport-level filtering blocks loopback, private IPv4/IPv6 ranges (RFC 1918), link-local, broadcast, multicast, documentation, benchmarking, and reserved IP literals across all extension network requests.
- **Extension Browser Egress Restrictions:** Mod actions `urls_to_open` and `clipboard_copies` now require explicit user button clicks (`button.is_some()`), preventing automated background triggers.
- **Browser URL Scheme Validation:** `open_browser` validates and accepts only `https://` URLs without control characters; native settings navigation (`ms-settings:`) is isolated to a hardcoded internal handler.
- **Extension Identifier Validation:** Enforced strict alphanumeric identifier validation across directory discovery and deletion routines to prevent path traversal.
- **Transactional Extension Extraction:** Zip installation now stages extraction through `.install-<id>` folders with a 1 MiB manifest size limit, safely rolling back if invalid or oversized entries are encountered.

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
- **Compiler Hygiene:** Resolved all compiler warnings and dead code annotations across public builds.
- **Driver Binaries Untracked:** Removed heavy binary driver installers and source archives from Git tracking to keep the repository lightweight.

### Security
- In-place updater network access is restricted strictly to official GitHub endpoints (`api.github.com`, `github.com`, `githubusercontent.com`).

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
- **Quick Sticky Notes:** In-notch notes tab with a two-column layout, five pastel color themes, auto-save, and direct export to Markdown or plain text. Includes a pop-out button (`â§‰ Pop`) that detaches a note into an independent, always-on-top desktop window.
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
