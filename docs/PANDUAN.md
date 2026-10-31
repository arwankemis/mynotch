# Panduan mynotch v0.0.2

## Instalasi

Unduh paket Windows x64 dari [Releases](https://github.com/arwankemis/mynotch/releases), ekstrak ke folder yang dapat ditulis, lalu jalankan `mynotch.exe`. Simpan `libunwind.dll` di folder yang sama.

## Kontrol notch

Aplikasi mulai pada Stage 1, berupa bar ringkas di tepi layar. Klik bar untuk membuka panel Stage 2. Mode minimal memakai notch kecil pada Stage 0; hover mengembalikan bar ringkas. Pin mempertahankan panel terbuka.

Gulir di Stage 0 atau 1 untuk mengubah volume master Windows. `Alt + N` membuka atau menutup notch; pintasannya dapat diubah di Pengaturan. `Esc` menutup pilihan atau tampilan aktif. Gunakan `Tab` dan `Shift + Tab` untuk memindahkan fokus, lalu `Enter` atau `Space` untuk mengaktifkan kontrol.

## Fitur

| Fitur | Penggunaan |
|---|---|
| Musik | Kontrol pemutar melalui sesi media Windows, seek, visualizer, dan lirik bila tersedia. Dukungan kontrol bergantung pada pemutar. |
| Kalender | Lihat tanggal, libur nasional Indonesia dan cuti bersama. Gulir untuk berganti bulan. |
| Reminder | Atur fokus/istirahat, hidrasi, peregangan, istirahat mata, dan pengingat sholat. Aplikasi harus tetap berjalan agar pengingat muncul. |
| Clipboard | Aktifkan melalui Tab & navigasi. Cari, pin, salin ulang, atau hapus riwayat teks. |
| Catatan | Pasang melalui Mod & Ekstensi. Buat catatan berwarna, ekspor `.md`/`.txt`, atau gunakan Pop untuk jendela desktop. |
| Cuaca | Pasang melalui Mod & Ekstensi. Pilih preset kota atau koordinat untuk kondisi saat ini dan prakiraan per jam. |

Persentase baterai laptop dan perangkat audio Bluetooth tampil bila Windows menyediakan datanya. Lirik, cuaca, dan jadwal sholat memerlukan koneksi jaringan saat mengambil data baru.

## Pengaturan

Klik ikon roda gigi. Perubahan disimpan otomatis.

- Umum: bahasa, startup Windows, perilaku fullscreen, dan mode minimal.
- Tampilan: posisi atas/bawah, skala, jarak tepi, opasitas, material, warna, dan gerakan.
- Tab & navigasi: aktifkan atau urutkan tab.
- Musik dan Reminder: sesuaikan perilaku fitur tersebut.
- Mod & Ekstensi: pasang, aktifkan, impor, atau lepas paket. [Panduan mod](PANDUAN-MOD.md).
- Pembaruan: periksa rilis dan unduh pembaruan.

Material Glass Lite dan Glass Adaptive bergantung pada dukungan Windows; aplikasi dapat memakai Solid bila backdrop tidak tersedia.

## Data dan privasi

Pengaturan dan catatan tersimpan di folder `data/` di sebelah executable. Clipboard mati secara default. Saat diaktifkan, riwayat disimpan sebagai teks biasa; entri biasa kedaluwarsa setelah 24 jam dan entri pin setelah 30 hari. Batas tiap entri adalah 64 KiB.

Indikator mikrofon memakai informasi sesi Windows. Deteksi kamera bersifat parsial dan eksperimental. Deteksi berbagi atau merekam layar belum didukung. [Data dan portabilitas](PORTABILITAS.md).

## Pembaruan

Pengaturan > Pembaruan memeriksa rilis GitHub, mengunduh paket lewat HTTPS, lalu memvalidasi checksum SHA-256 sebelum penggantian file. Ikuti pemberitahuan restart aplikasi untuk menerapkan pembaruan.

Dari v0.0.1.4 atau lebih lama, unduh dan ekstrak v0.0.2 secara manual sekali. Tutup aplikasi sebelum mengganti executable dan pertahankan folder `data/`. Pembaruan dalam aplikasi didukung mulai v0.0.2.

## Keluar

Klik kanan notch atau ikon tray, lalu pilih Keluar. Timer berhenti saat aplikasi ditutup.
