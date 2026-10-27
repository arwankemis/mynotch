# mynotch 0.0.1.1 (alpha)

*Notch bar native, estetik, dan super ringan untuk Windows 10 & 11.*

**mynotch** adalah aplikasi notch bar portabel yang bertengger rapi di sisi atas layar Anda. Didesain dengan estetika modern ala pulau dinamis (*dynamic island*), mynotch menemani hari kerja Anda dengan kontrol musik & lirik sinkron, kalender lengkap libur nasional Indonesia, pengingat harian (fokus, minum, peregangan, dan sholat), catatan cepat (*quick sticky notes*), serta pengelola clipboard — semuanya berjalan native, hemat memori, dan tanpa mengganggu pandangan layar Anda.

---

## ✨ Apa yang Baru di Versi 0.0.1.1 (alpha)?

Rilis **0.0.1.1 (alpha)** hadir membawa pembaruan besar berdasarkan hasil audit komprehensif dan masukan langsung pengguna:

- 📝 **Quick Sticky Notes (Fitur Baru!):** Catatan cepat langsung dari notch. Dilengkapi 5 palet warna pastel yang lembut, penyimpanan otomatis (*auto-save*) tanpa takut hilang, tombol *Pop to Desktop* untuk menjadikannya jendela catatan melayang mandiri di desktop, serta fitur ekspor instan ke format Markdown (`.md`) dan Plain Text (`.txt`).
- 🧩 **Sistem Mod & Ekstensi:** Arsitektur modular berbasis manifest yang bersih di Pengaturan. Fitur tambahan seperti Sticky Notes kini bisa dipasang (*install*) atau dilepas (*uninstall*) kapan saja tanpa membebani PC ketika tidak digunakan.
- 🌐 **Pilihan Bahasa Lengkap (Bilingual ID / EN):** Anda kini bisa memilih antara **Bahasa Indonesia** atau **English** di menu Pengaturan. Seluruh tab navigasi, tampilan kalender, submode pengingat, tombol, status, hingga notifikasi pop-up akan otomatis menyesuaikan bahasa pilihan Anda.
- 🧠 **Perilaku Stage 1 Cerdas (Smart Auto-revert):** Notch kini paham kapan harus tetap menampilkan info dan kapan harus kembali tenang. Saat mendengarkan musik dengan lirik aktif atau saat sesi fokus sedang berjalan, status akan terus tampil di notch. Sementara itu, untuk catatan baru, klip yang baru disalin, atau pengingat hidrasi/peregangan/sholat, notch akan menampilkan info selama 5 detik lalu bertransisi anggun kembali ke tampilan tanggal & jam.
- ⚡ **Peningkatan Kenyamanan UI/UX:**
  - Dukungan pintasan `Ctrl+A` (Select All) di seluruh kontrol teks input.
  - Dukungan roda gulir mouse (*mouse wheel*) untuk membalik bulan di Kalender dan halaman di Clipboard.
  - Tombol *"Ikuti"* (*Follow*) di tab Musik untuk kembali ke lirik lagu yang sedang aktif setelah Anda melakukan scroll manual.
  - Mode Minimal yang disempurnakan menjadi kapsul hitam ramping yang elegan.
  - Hover delay 140 ms yang halus agar notch tidak sensitif terbuka tanpa sengaja saat kursor lewat sepintas.

---

## 🚀 Mulai Memakai

mynotch adalah aplikasi **100% portabel** — tidak membutuhkan instalasi yang rumit, tidak memerlukan hak akses administrator, dan tidak mengotori registry Windows Anda.

1. **Unduh Paket:** Ambil berkas `mynotch-0.0.1.1-alpha-windows-x64.zip` dari halaman [Releases](https://github.com/arwankemis/mynotch/releases).
2. **Ekstrak Berkas:** Ekstrak seluruh isi arsip ZIP ke folder yang dapat ditulis (misalnya di folder `Documents\mynotch` atau folder utilitas pribadi Anda).
3. **Jalankan:** Klik dua kali pada `mynotch.exe`. Pastikan berkas `libunwind.dll` tetap berada di folder yang sama di samping executable.
4. **Gunakan:**
   - Arahkan kursor (*hover*) ke notch di bagian atas layar untuk melirik informasi ringkas (*Stage 1*).
   - Klik notch untuk membuka panel utama (*Stage 2*).
   - Klik ikon gerigi untuk membuka menu Pengaturan.

> **Tips Portabilitas:** Semua data konfigurasi, catatan, dan riwayat clipboard tersimpan rapi di dalam folder `data` di samping `mynotch.exe`. Jika ingin berganti laptop atau PC, cukup salin seluruh folder mynotch ke PC baru Anda!

---

## 🎛️ Fitur Utama

### 🎵 Musik & Lirik
- Terhubung otomatis dengan pemutar media Windows (Spotify, YouTube Music via browser, Apple Music, Media Player, dll.).
- Kendali pemutar lengkap: putar/jeda, lagu sebelumnya, lagu berikutnya, dan scrubber waktu lagu.
- Visualizer spektrum audio real-time berbasis hardware.
- Lirik lagu tersinkronisasi (*karaoke-style*) dan tombol *Follow* untuk kembali ke baris aktif.

### 📅 Kalender Indonesia & Libur Nasional
- Kalender bulanan yang bersih dengan navigasi mudah via klik atau scroll mouse.
- Dilengkapi penanda resmi Hari Libur Nasional & Cuti Bersama Indonesia (dataset 2026).
- Panel info harian di sebelah kanan: cek apakah hari ini adalah hari kerja biasa atau akhir pekan, lengkap dengan kartu jadwal libur nasional berikutnya.

### 📝 Quick Sticky Notes (Mod Catatan Cepat)
- Simpan memo, to-do list kilat, atau ide mendadak tanpa perlu membuka aplikasi berat.
- 5 warna pastel yang nyaman dipandang (Kuning, Biru, Hijau, Pink, Ungu).
- Fitur *Pop to Desktop*: klik tombol `⧉ Pop` untuk memunculkan catatan menjadi jendela melayang di atas desktop dengan opsi pin selalu di depan (*always-on-top*).
- Simpan dan bagikan catatan dengan satu klik: ekspor ke Markdown (`.md`), Plain Text (`.txt`), atau salin langsung ke clipboard.

### ⏰ Pengingat Harian (Smart Reminders)
- 🎯 **Fokus:** Timer pomodoro terarah dengan durasi fokus dan jeda istirahat yang dapat disesuaikan.
- 💧 **Hidrasi:** Pelacak asupan air harian dengan target volume (ml) dan pengingat minum teratur.
- 🧘 **Peregangan:** Pengingat relaksasi mata dan peregangan tubuh setelah duduk bekerja dalam waktu lama.
- 🕌 **Sholat:** Jadwal sholat 5 waktu akurat untuk kota-kota di Indonesia (Jakarta, Bandung, Surabaya, dan lainnya) melalui integrasi myQuran berbasis WIB, dilengkapi notifikasi tepat waktu atau beberapa menit sebelumnya.

### 📋 Pengelola Clipboard (Opsional)
- Menyimpan riwayat teks yang pernah Anda salin secara lokal.
- Salin ulang hanya dengan sekali klik atau hapus klip lama yang tidak diperlukan.
- **Aman & Menjaga Privasi:** Format clipboard sensitif dari Password Manager (seperti Bitwarden, 1Password, KeePass) secara otomatis diabaikan dan tidak akan disimpan.

---

## ⌨️ Pintasan Keyboard

| Pintasan | Fungsi |
|---|---|
| `Alt + N` | Buka atau tutup panel notch dari mana saja secara instan |
| `Ctrl + A` | Blok seluruh teks di dalam kotak judul memo, isi catatan, atau kotak input |
| `Esc` | Tutup dropdown menu yang aktif atau kembali dari Pengaturan ke tab sebelumnya |
| `Space` / `Enter` | Klik tombol yang sedang memiliki fokus keyboard |
| `Tab` / `Shift + Tab` | Berpindah antar kontrol tombol dan input secara berurutan |
| `Roda Mouse (Scroll)` | Ganti bulan di kalender atau ganti halaman di riwayat clipboard |

---

## ⚙️ Kategori Pengaturan

Navigasi pengaturan tersusun rapi di sebelah kiri dengan kontrol di sisi kanan:

- **Umum:** Mode layar penuh, mode bilah kecil (*minimal mode*), pilihan tab awal saat startup, dan pengaturan bahasa antarmuka (Bahasa Indonesia / English).
- **Tampilan:** Posisi bilah (atas atau bawah layar), skala antarmuka, jarak dari tepi layar, dan tingkat kepekatan/transparansi latar.
- **Tab & Navigasi:** Mengatur urutan tab (bisa di-drag/reorder) serta mengaktifkan/menonaktifkan tab tertentu.
- **Musik:** Pengaturan visualizer spektrum audio, warna latar cover art, dan opsi romanisasi lirik alfabet.
- **Reminder:** Ringkasan status dan konfigurasi cepat untuk empat pilar pengingat harian Anda.
- **Mod & Ekstensi:** Kelola modul tambahan seperti Quick Sticky Notes.

---

## 💻 Kebutuhan Sistem & Spesifikasi

- **Sistem Operasi:** Windows 10 atau Windows 11 (64-bit / x64).
- **Grafis & Render:** Mendukung Direct3D 11 / Direct2D hardware-accelerated. Jika driver grafis terbatas, mynotch otomatis beralih ke software renderer (WARP) yang aman dan stabil.
- **Ukuran File & Efisiensi:** Hanya berkisar ~1.1 MB, dibangun murni menggunakan Rust native tanpa runtime WebView atau Electron yang memakan ratusan megabyte RAM.

---

## 📖 Dokumentasi Lengkap

- [Panduan Pemakaian](docs/PANDUAN.md) — Panduan mendalam penggunaan setiap fitur dan tombol.
- [Data dan Portabilitas](docs/PORTABILITAS.md) — Penjelasan detail penyimpanan lokal dan cara backup data Anda.
- [Build dan Pengembangan](docs/PENGEMBANGAN.md) — Panduan kompilasi kode sumber menggunakan Rust toolchain.
- [Catatan Perubahan Versi (Changelog)](CHANGELOG.md) — Riwayat lengkap pembaruan per rilis.
- [Lisensi Pihak Ketiga](THIRD_PARTY_NOTICES.md) — Atribusi lisensi pustaka open-source yang digunakan.

---

*Dibuat dengan dedikasi untuk produktivitas yang nyaman, hening, dan menyenangkan di desktop Windows Anda.*
