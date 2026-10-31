# Third-party notices

This private alpha application uses the components below. Their license texts are included under licenses/. The application has not established a license for public redistribution of its own source.

| Package | Version | License expression |
|---|---|---|
| autocfg | 1.5.1 | Apache-2.0 OR MIT |
| chrono | 0.4.45 | MIT OR Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| syn | 2.0.119 | MIT OR Apache-2.0 |
| syn | 3.0.9 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| windows | 0.62.2 | MIT OR Apache-2.0 |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 |
| windows-core | 0.62.2 | MIT OR Apache-2.0 |
| windows-future | 0.3.2 | MIT OR Apache-2.0 |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 |
| windows-result | 0.4.1 | MIT OR Apache-2.0 |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 |
| zmij | 1.0.23 | MIT |

## Runtime

libunwind.dll is bundled from the Rust/LLVM GNU Windows toolchain used for this build. Toolchain runtime license texts, including Apache-2.0, MIT, LLVM exception and NCSA texts, are included under licenses/rust-runtime/. No compiler or development toolchain is included in the portable ZIP. Windows system APIs remain provided by Windows.

## Online data

Lyrics are requested from LRCLIB (https://lrclib.net/). Prayer schedules are requested from myQuran (https://api.myquran.com/doc), whose documentation attributes the schedule data to Kemenag Bimas Islam. The app does not claim ownership of lyrics or external schedule data.


## Optional bundled power sensor reader

The Public and Personal distributions include `power-provider/`, used only while Mod Daya is active. It contains self-contained .NET 10.0.12, LibreHardwareMonitorLib 0.9.6 (MPL-2.0), its transitive libraries, and the official signed PawnIO 2.1.0 setup. PawnIO.Modules uses LGPL terms; the driver uses GPL terms with its upstream exception. Full notices and exact corresponding source archives are distributed in `power-provider/notices/` and `power-provider/sources/`; pinned versions, source commits and SHA-256 values are in `power-provider/source-manifest.json`. The mynotch helper's complete source/project/lock file is in `power-provider/source/`.

Driver setup is optional and user initiated through Daya's CPU sensor setup button. It is never silently installed. The main mynotch UI is not elevated and no Windows protection settings are changed. See the bundled provider README for supported platforms, per-session CPU permission and measured-component limitations.
