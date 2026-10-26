# mynotch 0.0.1 alpha

Aplikasi notch native untuk Windows dengan musik, kalender Indonesia, dan pengingat harian. Versi alpha ini **portable** dan repository ini disiapkan sebagai proyek private.

## Mulai memakai

1. Unduh paket `mynotch-0.0.1-alpha-windows-x64.zip` dari halaman Releases dan ekstrak seluruh isinya.
2. Simpan folder pada lokasi yang dapat ditulis, misalnya Documents atau folder aplikasi pribadi.
3. Jalankan `mynotch.exe`. Biarkan `libunwind.dll` berada di samping executable.
4. Hover untuk membuka notch kecil, klik untuk membuka panel. Ikon pengaturan membuka menu Pengaturan baru.

Tidak memerlukan installer atau hak administrator. Data dibuat otomatis di folder `data` di samping executable. Untuk pindah PC, salin seluruh folder aplikasi beserta `data` jika ingin membawa pengaturan dan riwayat.

## Fitur alpha

- **Musik:** metadata media Windows, kontrol putar/jeda/sebelumnya/berikutnya, sampul, lirik, dan visualizer ketika tersedia. Ini bukan layanan identifikasi lagu dari rekaman suara.
- **Kalender:** kalender bulanan, libur nasional/cuti bersama dari dataset 2026, dan detail tanggal. Tahun tanpa dataset diberi keterangan belum tersedia.
- **Reminder:** Fokus, Hidrasi, Peregangan, dan Sholat. Default lokasi Sholat adalah Jakarta; pilihan kota lain Bandung dan Surabaya. Jadwal otomatis melalui myQuran, mengikuti WIB, dengan cache dan validasi kota/tanggal.
- **Clipboard opsional:** diaktifkan melalui Pengaturan → Tab & navigasi. Default mati. Ketika diaktifkan, riwayat teks tersimpan lokal di folder data.

## Pengaturan yang baru

Navigasi di kiri, kontrol terkait di kanan:

| Menu | Isi |
|---|---|
| Umum | Perilaku layar penuh, mode bar kecil, tab awal |
| Tampilan | Atas/bawah layar, ukuran UI, jarak tepi, kepekatan latar |
| Tab & navigasi | Aktif/mati dan urutan tab; minimal satu tab aktif |
| Musik | Visualizer, warna sampul, akses ke pemutar |
| Reminder | Ringkasan empat pengingat dan tombol Atur ke panel masing-masing |

Perubahan disimpan otomatis. Esc menutup dropdown, lalu kembali dari Pengaturan ke tab sebelumnya. Tab/Shift+Tab dan Enter menavigasi kontrol; dropdown juga mendukung tombol arah, Home/End, Enter, dan Esc. Detail reminder berada pada satu tempat, bukan diduplikasi di menu Pengaturan.

## Sistem dan batas alpha

Ditujukan untuk Windows 10/11 x64 dengan dukungan Direct3D 11. Renderer mencoba hardware dan beralih ke software WARP bila pembuatan renderer hardware gagal. Opsi `--software-renderer` tersedia untuk pengujian atau troubleshooting. Pengujian dilakukan di PC pengembangan dan melalui pemindahan folder; seluruh kombinasi hardware/Windows belum diuji.

Aplikasi belum ditandatangani digital. Media/lirik bergantung pada dukungan aplikasi pemutar dan koneksi; lirik tidak selalu tersedia. Pengingat hanya berjalan selama aplikasi aktif. Opsi suara Sholat memakai bunyi sistem, belum rekaman adzan. Build ini tidak memasang startup Windows otomatis dan tidak memerlukan aplikasi khusus PC pembuatnya.

## Dokumentasi

- [Panduan pemakaian](docs/PANDUAN.md)
- [Data dan portabilitas](docs/PORTABILITAS.md)
- [Build dan pengembangan](docs/PENGEMBANGAN.md)
- [Perubahan versi](CHANGELOG.md)
- [Dependensi dan lisensi pihak ketiga](THIRD_PARTY_NOTICES.md)

Data runtime, cache, log, dan profil pengujian tidak termasuk dalam repository atau ZIP rilis. Jaga folder data ketika membagikan aplikasi: folder itu dapat memuat riwayat clipboard dan catatan pribadi.
