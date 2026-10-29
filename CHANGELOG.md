# Changelog

Semua pembaruan penting dan catatan rilis untuk mynotch didokumentasikan di berkas ini.

---

## [0.0.1.3 alpha] - 29 Oktober 2026

Rilis 0.0.1.3 alpha menambahkan dukungan mod kustom buatan pengguna berbasis deklaratif JSON dan skrip Lua 5.4, tombol pemasang paket mod di menu Pengaturan, pintasan folder mod ke Windows Explorer, serta rendering kartu interaktif.

### Fitur baru

- Pemasang mod mandiri:
  - Tombol "+ Pasang Mod" di menu Pengaturan Mod untuk memilih paket `.zip` atau berkas manifest `.json`.
  - Ekstraksi aman paket ZIP langsung ke direktori `data/mods/` tanpa dependensi eksternal.
  - Tombol "Folder Mod" untuk membuka direktori `data/mods/` langsung di Windows Explorer.
  - Tombol "Hapus" pada daftar mod untuk menghapus berkas mod buatan sendiri dari disk.
- Mesin mod skrip Lua 5.4:
  - Mesin virtual pure Rust (Piccolo) yang berjalan di lingkungan sandbox terisolasi tanpa akses I/O file atau jaringan bebas.
  - API antarmuka pengguna: `ui.card`, `ui.button`, dan `ui.text`.
  - API sistem: `system.toast`, `system.open_url`, dan `system.copy`.
  - Perlindungan perulangan tak terbatas berbasis alokasi instruksi (fuel limits).
- Rendering dinamis tab mod:
  - Mod yang diaktifkan otomatis terdaftar pada bar navigasi notch.
  - Tampilan grid responsif untuk kartu ringkasan, teks kustom, dan tombol interaktif.
  - Kotak diagnostik kesalahan skrip jika mod Lua mengalami kegagalan saat dieksekusi.
- Berkas contoh dan dokumentasi:
  - Contoh mod deklaratif kurs valuta asing di `examples/mods/kurs_usd/`.
  - Contoh mod interaktif skrip Lua di `examples/mods/counter_lua/`.
  - Buku panduan pembuatan dan pengemasan mod di `docs/PANDUAN-MOD.md`.

---

## [0.0.1.2 alpha] - 28 Oktober 2026

Rilis 0.0.1.2 alpha menambahkan kontrol volume master lewat scroll mouse, indikator baterai laptop, mod cuaca Open-Meteo, pencarian dan pin riwayat clipboard, opsi startup Windows, serta pemeriksa rilis GitHub.

### Fitur baru

- Kontrol volume pada notch: gulirkan roda mouse di atas notch (Stage 0 dan Stage 1) untuk mengatur volume master Windows via Core Audio. HUD volume muncul sementara menampilkan persentase dan bar level.
- Pemantau baterai di Stage 1: menampilkan persentase baterai laptop, ikon pengisian daya, dan peringatan baterai lemah di samping jam. Notifikasi toast muncul saat kabel adaptor terpasang atau dilepas.
- Mod cuaca Open-Meteo:
  - Panel native 560 x 240 dengan informasi suhu, kondisi bilingual, suhu terasa, kelembapan, kecepatan angin, peluang hujan, dan prakiraan per jam hingga empat jam ke depan.
  - Background bergradasi dari hitam notch mengikuti kondisi langit serta status siang atau malam.
  - Ikon MGC Weather dimuat dari atlas offline via WIC Direct2D (gaya gradasi pada panel dan prakiraan per jam; gaya color fill pada Stage 1).
  - Pilihan kota preset untuk Jakarta, Bandung, dan Surabaya, serta formulir koordinat lintang dan bujur kustom.
  - Refresh data dapat dipicu dengan klik tombol atau scroll mouse pada tab cuaca. Cache 30 menit dan pembatasan frekuensi refresh menjaga efisiensi jaringan. Mod bersifat opsional dan dapat dilepas lewat Pengaturan.
- Pengelola clipboard:
  - Tombol pin untuk menyematkan klip penting ke urutan teratas riwayat.
  - Kolom pencarian instan untuk menyaring riwayat teks saat mengetik.
- Jalankan saat startup Windows: opsi di menu Pengaturan > Umum yang memanfaatkan registry pengguna (HKCU) tanpa service latar belakang.
- Pemeriksa pembaruan dalam aplikasi: menu Pembaruan di Pengaturan untuk memeriksa rilis baru dari GitHub, menampilkan versi terpasang, tag rilis baru, ukuran berkas, serta tautan unduh paket.

### Perbaikan dan penyempurnaan

- Normalisasi layout Stage 1:
  - Menyamakan format status kalender, cuaca, dan musik dengan gap 8 px di kedua sisi pemisah `|`.
  - Teks informasi panjang dipotong dengan ellipsis sebelum pemisah agar jam dan baterai tetap terbaca.
  - Ikon Stage 1 ikut bergeser bersama grup teks saat dipusatkan.
  - Transisi perubahan ukuran compact menggunakan morph 260 ms yang mengambil sampel posisi saat berjalan.
- Penanganan status musik sementara: pergantian lagu memunculkan status musik di Stage 1 selama lima detik tanpa mengubah tab pilihan pengguna, lalu kembali ke status sebelumnya setelah waktu habis.
- Pemetaan ikon navigasi tab cuaca dipulihkan ke ikon awan (cloud).
- Penyesuaian lebar Stage 1 untuk mengakomodasi indikator baterai laptop di semua tab aktif tanpa memotong teks tanggal.
- Sinkronisasi label versi 0.0.1.2 alpha pada deskripsi menu Pengaturan Umum dan skrip pembuatan paket rilis.

---

## [0.0.1.1 alpha] - 27 Oktober 2026

Rilis 0.0.1.1 alpha menambahkan catatan cepat (quick sticky notes), sistem mod modular, dukungan bilingual Indonesia dan English, serta perbaikan navigasi dan bug UI.

### Fitur baru

- Quick Sticky Notes:
  - Tab Catatan pada notch dengan tata letak dua kolom (daftar memo di kiri dan editor teks di kanan).
  - Lima pilihan warna untuk mengelompokkan catatan.
  - Tombol Pop to Desktop untuk melepas catatan menjadi jendela melayang di desktop dengan pin always-on-top.
  - Ekspor catatan ke format Markdown (`.md`) atau teks biasa (`.txt`), serta tombol salin langsung ke clipboard.
  - Penyimpanan otomatis tanpa tombol simpan manual.
- Mod modular: menu Mod & Ekstensi di Pengaturan untuk memasang atau melepas fitur tambahan berbasis manifest (`.mod.json`).
- Dukungan bilingual: pilihan bahasa Indonesia dan English di Pengaturan yang langsung diterapkan ke seluruh judul tab, kalender, pengingat, dan teks tombol.
- Transisi status Stage 1: status musik aktif dan timer fokus tetap tampil di Stage 1, sedangkan catatan baru, klip tersalin, atau pengingat sholat tampil selama 5 detik sebelum kembali ke tampilan tanggal dan jam.

### Perbaikan dan kenyamanan antarmuka

- Dukungan tombol `Ctrl+A` di seluruh kotak input teks.
- Dukungan scroll mouse untuk berpindah bulan di kalender dan halaman di clipboard.
- Tombol Ikuti pada pemutar musik untuk kembali ke baris lirik aktif setelah digulir manual.
- Mode minimal disesuaikan menjadi kapsul hitam solid.
- Penambahan jeda hover 140 ms agar notch tidak terbuka saat kursor mouse lewat sepintas.
- Penyesuaian teks pengingat sholat menjadi "Tepat waktu" saat lead time bernilai 0 menit.
- Mengklik kartu libur nasional berikutnya langsung membuka tanggal terkait di kalender.
- Penanda fokus keyboard dan dukungan penekanan tombol Space atau Enter pada kontrol interaktif.

### Perbaikan bug

- Mengatasi draf catatan baru yang sempat menampilkan teks dari catatan sebelumnya setelah penghapusan.
- Memperbaiki pembungkusan kata DirectWrite agar teks panjang tidak meluap keluar batas panel.
- Memperbaiki pelepasan handle GDI dan alokasi memori internal ICU pada pengurai lirik dan clipboard.
- Mengabaikan format clipboard sensitif dari aplikasi pengelola kata sandi.
- Memastikan menu pop-up pengaturan dan pengingat tertutup saat notch berpindah mode.
- Memperbaiki prioritas notifikasi toast untuk penyelesaian sesi fokus.

---

## [0.0.1 alpha] - 26 Oktober 2026

- Rilis perdana mynotch dengan executable `mynotch.exe` untuk Windows x64.
- Integrasi kontrol media musik, lirik sinkron, dan visualizer spektrum audio WASAPI.
- Kalender bulanan dengan penanda resmi Hari Libur Nasional Indonesia 2026.
- Pengingat harian: fokus Pomodoro, hidrasi, peregangan tubuh, dan jadwal sholat berbasis data myQuran (WIB).
- Riwayat teks clipboard lokal opsional.
- Tata letak Pengaturan dua kolom.
- Dukungan fallback Direct3D software WARP saat inisialisasi Direct2D perangkat keras membutuhkan cadangan.
