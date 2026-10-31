# Data dan portabilitas

mynotch menyimpan data relatif terhadap lokasi executable:

```text
mynotch/
  mynotch.exe
  libunwind.dll
  data/
    settings.json
    state.json
    cache/
```

Gunakan folder yang dapat ditulis. Untuk berpindah komputer atau membuat cadangan, tutup aplikasi lalu salin folder aplikasi beserta `data/`. Opsi startup Windows dapat diaktifkan dari Pengaturan dan memakai lokasi executable saat itu; periksa kembali opsi tersebut setelah memindahkan aplikasi.

## Data lokal

Folder `data/` dapat memuat pengaturan, catatan, riwayat hidrasi, cache lirik dan jadwal, data mod, serta riwayat clipboard bila diaktifkan. Clipboard disimpan sebagai teks biasa dalam `state.json`, bukan dienkripsi. Entri biasa kedaluwarsa setelah 24 jam, entri pin setelah 30 hari, dengan batas 64 KiB per entri.

Aplikasi menghormati penanda pengecualian clipboard yang didukung, tetapi tidak dapat mengenali semua teks sensitif. Jangan membagikan folder data ketika mengirim aplikasi atau laporan bug.

## Koneksi jaringan

| Fitur | Tujuan |
|---|---|
| Lirik | LRCLIB, memakai metadata lagu untuk pencarian. |
| Jadwal sholat | myQuran, memakai lokasi yang dipilih. |
| Cuaca | Open-Meteo, memakai koordinat lokasi yang dipilih. |
| Pembaruan | GitHub dan endpoint unduhan rilis resmi. |
| Mod komunitas | Alamat yang diminta mod setelah izin jaringan diberikan. |

Kalender libur 2026 disertakan dalam aplikasi. Musik membaca sesi media Windows; visualizer membaca output audio saat dibutuhkan dan tidak merekam mikrofon.

## Indikator privasi

Indikator mikrofon membaca informasi sesi CoreAudio tanpa membuka stream mikrofon. Deteksi kamera memakai informasi aktivitas Windows dan bersifat parsial/eksperimental; status dapat terlambat atau tidak lengkap. Aplikasi tidak membaca gambar kamera untuk indikator ini.

Pemantauan aktivitas berbagi layar dan perekaman layar aplikasi lain belum didukung. Indikator tidak menggantikan pengaturan izin dan privasi Windows.
