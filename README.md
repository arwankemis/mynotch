# mynotch

Aplikasi notch bar portabel untuk Windows 10 dan 11 yang dibangun dengan Rust, Direct2D, dan DirectWrite.

Proyek ini dikembangkan melalui proses vibe coding, terinspirasi dari aplikasi notch Windows seperti Nusa dan Poni. mynotch memindahkan fungsi cepat ke bilah atas layar tanpa browser atau runtime Electron.

## Cara kerja notch

mynotch menggunakan tiga tingkat tampilan:

- Stage 0: bilah minimal di tepi layar saat diminimalkan.
- Stage 1: bilah status ringkas yang menampilkan jam, status baterai laptop, baterai TWS/headset Bluetooth yang terhubung, serta status tab aktif (kalender, musik, atau cuaca).
- Stage 2: panel interaktif lengkap yang terbuka saat notch diklik.

Memutar roda mouse di atas bilah notch (Stage 0 dan Stage 1) mengatur volume master Windows. Selama notch terlihat, mynotch menahan OSD volume Windows dan menampilkan HUD-nya sendiri. HUD yang sama muncul saat volume diubah dari tombol media, tombol HID laptop, atau aplikasi lain. Kalau notch disembunyikan di fullscreen, Windows kembali menampilkan OSD-nya.

## Fitur

### Deteksi TWS dan headset Bluetooth
mynotch memantau perangkat audio Bluetooth yang sedang terhubung (TWS, headset, atau speaker). Saat perangkat tersambung atau terputus, notch menampilkan notifikasi berisi nama perangkat, dan Stage 1 menampilkan ikon 🎧 beserta persentase baterai bila Windows menyediakan datanya. Pemeriksaan berjalan di latar belakang setiap 10 detik sehingga tidak mengganggu animasi notch.

### Indikator volume
mynotch membaca dan mengubah volume master lewat Core Audio. Selama notch terlihat, jendela OSD volume Windows ditahan minimized, jadi hanya HUD mynotch yang muncul. HUD dipicu oleh perubahan volume dari jalur mana pun: scroll di notch, tombol media, tombol HID laptop, atau aplikasi lain. Di fullscreen yang menyembunyikan notch, OSD Windows dibiarkan bekerja.

### Musik dan lirik
Aplikasi terhubung dengan Windows Media Transport Controls (Spotify, YouTube Music di browser, Apple Music, dan pemutar media lainnya). Panel musik menyediakan kendali putar, jeda, lompat lagu, visualizer audio, serta lirik terjemahan atau romanisasi CJK. Pergantian lagu memunculkan status judul lagu di Stage 1 selama lima detik sebelum kembali ke tampilan awal.

### Mod cuaca
Mod cuaca berbasis Open-Meteo menampilkan suhu saat ini, kondisi langit, suhu terasa, kelembapan, kecepatan angin, peluang hujan, dan prakiraan per jam hingga empat jam ke depan. Background panel bergradasi mengikuti kondisi siang, malam, berawan, atau hujan. Lokasi dapat dipilih dari preset (Jakarta, Bandung, Surabaya) atau koordinat lintang dan bujur manual. Mod ini opsional dan dapat dipasang atau dilepas lewat menu Pengaturan.

### Kalender dan libur nasional
Kalender bulanan dilengkapi daftar hari libur nasional serta cuti bersama Indonesia tahun 2026. Navigasi bulan dapat dilakukan dengan klik tombol atau scroll mouse.

### Pengingat harian
Tab reminder menyediakan empat pengingat:
- Sesi fokus Pomodoro dengan durasi kerja dan jeda terpisah.
- Catatan hidrasi harian dengan target volume air.
- Pengingat peregangan tubuh dan jeda mata setelah bekerja.
- Jadwal sholat lima waktu untuk wilayah Indonesia berbasis koordinat dan WIB, lengkap dengan notifikasi pengingat.

### Catatan cepat
Mod catatan sticky menyediakan memo cepat dengan lima pilihan warna. Catatan dapat dilepas menjadi jendela melayang di desktop (pop to desktop), disalin ke clipboard, atau diekspor ke berkas Markdown (`.md`) dan teks biasa (`.txt`).

### Sistem mod dan skrip Lua
mynotch mendukung mod buatan komunitas yang dapat dipasang langsung dari menu Pengaturan. Pengguna dapat memilih berkas arsip `.zip` atau `.json` menggunakan tombol "Impor Mod", atau membuka folder penyimpanan dengan tombol "Folder Mod". Tiap mod punya tombol Buka, Pasang/Lepas, dan Hapus, plus toggle Tab dan Net. Mod dapat dibuat secara deklaratif lewat format JSON sederhana atau dinamis menggunakan skrip Lua 5.4 yang berjalan di lingkungan sandbox terisolasi. Contoh mod tersedia di `examples/mods/`: kurs USD, counter Lua, dan monitor IHSG. Panduan pembuatan mod ada di [Panduan Mod](docs/PANDUAN-MOD.md).

### Pengelola clipboard
Mencatat riwayat teks yang disalin secara lokal. Item dapat disematkan ke posisi teratas dengan tombol pin. Tersedia kotak pencarian teks untuk menyaring riwayat. Format sensitif dari pengelola kata sandi diabaikan otomatis.

### Pengaturan dan sistem
- Antarmuka mendukung Bahasa Indonesia dan English tanpa perlu memulai ulang aplikasi.
- Opsi untuk menjalankan aplikasi otomatis saat Windows login melalui registry pengguna.
- Pembaruan aplikasi dapat diperiksa langsung dari menu Pengaturan yang terhubung ke GitHub Releases.
- Posisi notch (atas atau bawah layar), skala tampilan, jarak tepi, dan kepekatan latar dapat diatur sesuai kebutuhan.

## Penggunaan

mynotch berupa berkas portabel tanpa instalasi:

1. Unduh arsip rilis `mynotch-0.0.1.3-alpha-windows-x64.zip` dari halaman [Releases](https://github.com/arwankemis/mynotch/releases).
2. Ekstrak isi ZIP ke folder pilihan Anda.
3. Jalankan `mynotch.exe`. Simpan berkas `libunwind.dll` di folder yang sama di samping executable.

Semua konfigurasi, catatan, dan riwayat clipboard disimpan di dalam folder `data/` di samping berkas executable. Untuk memindahkan aplikasi ke perangkat lain, cukup salin folder tersebut.

## Pintasan keyboard

| Pintasan | Fungsi |
|---|---|
| `Alt + N` | Buka atau tutup panel notch |
| `Ctrl + A` | Pilih semua teks pada kotak input |
| `Esc` | Tutup dropdown aktif atau kembali ke tab sebelumnya di Pengaturan |
| `Space` / `Enter` | Tekan tombol yang sedang aktif |
| `Tab` / `Shift + Tab` | Pindah fokus antar elemen |
| `Scroll Mouse` | Ganti bulan kalender, halaman clipboard, atau ubah volume saat berada di bilah notch |

## Kebutuhan sistem

- Windows 10 atau Windows 11 (64-bit).
- Kartu grafis dengan dukungan Direct3D 11 dan Direct2D (otomatis beralih ke software WARP jika akselerasi perangkat keras tidak tersedia).

## Dokumentasi

- [Panduan Pengguna](docs/PANDUAN.md): petunjuk penggunaan tiap modul dan kontrol.
- [Panduan Pembuatan Mod](docs/PANDUAN-MOD.md): petunjuk pembuatan mod deklaratif JSON dan skrip Lua.
- [Catatan Rilis dan Perubahan](CHANGELOG.md): riwayat perubahan tiap versi.
- [Data dan Portabilitas](docs/PORTABILITAS.md): struktur penyimpanan data lokal dan prosedur pencadangan.
- [Kompilasi dan Pengembangan](docs/PENGEMBANGAN.md): panduan build menggunakan Rust toolchain.
- [Lisensi Pihak Ketiga](THIRD_PARTY_NOTICES.md): atribusi pustaka sumber terbuka yang digunakan.
