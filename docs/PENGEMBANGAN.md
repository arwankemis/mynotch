# Build dari source

## Prasyarat

- Windows x64 dan PowerShell 5.1 atau lebih baru.
- Rust stable dengan dukungan edition 2024 dan target Windows yang sesuai. Dependensi dikunci dalam `source/Cargo.lock`.
- Python 3 untuk pemeriksaan isi sebelum packaging.
- .NET 10 SDK untuk membangun hardware sensor provider.

## Build publik

Jalankan dari root repositori:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build.ps1
```

Target default adalah `x86_64-pc-windows-gnullvm`. Untuk MSVC:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Target x86_64-pc-windows-msvc
```

Executable publik ditulis ke `dist/public/mynotch.exe` dan disalin ke root repositori. Versi tampilan rilis ditentukan oleh `release/version.json`.

## Pengujian

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Test
```

Sebagian pengujian perangkat atau jaringan memerlukan lingkungan yang sesuai. Untuk pemeriksaan tampilan terisolasi, `mynotch.exe --preview` memakai folder `preview-data/` terpisah dari data normal.

## Packaging

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package.ps1
```

Paket publik menyertakan executable, runtime yang diperlukan, panduan pengguna, contoh mod, serta lisensi. Panduan build ini hanya tersedia di repositori source dan tidak dimasukkan ke ZIP pengguna. Packaging memeriksa isi folder staging dan ZIP akhir; temuan yang ditolak harus diperbaiki sebelum publikasi. Jangan sertakan `data/`, `preview-data/`, log, atau konfigurasi akun dalam distribusi.

## Struktur

| Folder | Isi |
|---|---|
| `source/` | Aplikasi Rust dan source sensor provider. |
| `scripts/` | Build, test, dan packaging. |
| `docs/` | Panduan publik dan screenshot. |
| `examples/` | Contoh mod. |
| `licenses/` | Lisensi komponen yang dipaketkan. |
| `dist/` | Hasil build lokal; tidak dilacak Git. |
