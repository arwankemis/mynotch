# Bundled mynotch power reader

Windows x64, self-contained .NET 10.0.12 and unmodified LibreHardwareMonitorLib 0.9.6. No separately installed .NET runtime or LibreHardwareMonitor application is required. Source is in this folder; exact upstream sources, licenses and the signed official PawnIO setup are beside it. Dependencies and driver SHA-256 are pinned.

The normal reader enables only CPU/GPU monitoring, samples every two seconds and sends bounded schema-1 JSON lines through private process stdout. It does not run a network listener. Closing stdin stops it. mynotch starts it only while Daya is active and stops it on disable/exit.

GPU readings normally use vendor interfaces directly. CPU sensors may require administrator access and the PawnIO driver. Click **Set up CPU sensor / Siapkan sensor CPU** in Daya and approve Windows UAC yourself. This starts a CPU-only read session connected through a random same-user-only named pipe; the main mynotch window remains unelevated. If the driver is absent, the included official signed installer opens first. No Windows protection setting is changed. This is a per-session grant, not a permanently installed privileged mynotch service. Closing the mod/session closes the elevated reader's pipe. Cancelling setup leaves available GPU readings working.

Zero/invalid power readings are shown as unavailable. Sensor support varies by hardware; setup cannot create sensors the hardware/driver does not expose. CPU/GPU subtotal is not wall/AC power. AC current, voltage and whole-PC electricity require a suitable external meter.

Rebuild: `dotnet restore -r win-x64 --locked-mode`, then `dotnet publish -c Release -r win-x64 --self-contained true --no-restore`. The repository's `scripts/build-power-provider.ps1` assembles all runtime files, notices and corresponding source archives. `mynotch-power.exe --self-test` checks selection/protocol rules; `--once` probes actual sensors. `--stream` requires an open stdin; only the fixed `cpu-admin` command requests CPU setup. Internal elevated mode validates a generated pipe name and exposes no arbitrary command interface.

Upstream: https://github.com/LibreHardwareMonitor/LibreHardwareMonitor (MPL-2.0), https://github.com/namazso/PawnIO.Modules (LGPL), https://github.com/namazso/PawnIO (GPL with upstream exception). See `source-manifest.json`, `notices/` and `sources/` for exact versions, full terms and source commits, including transitive MPL dependencies.
