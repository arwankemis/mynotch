# Changelog

Semua pembaruan penting dan catatan rilis untuk **mynotch** didokumentasikan di berkas ini.

---

## [0.0.1.1 alpha] — 27 Oktober 2026

Rilis **0.0.1.1 (alpha)** adalah tonggak penting dalam penyempurnaan `mynotch`. Di versi ini, kami menindaklanjuti seluruh temuan audit menyeluruh terhadap UI/UX, memperbaiki bug fungsional, menambahkan fitur *Quick Sticky Notes* yang telah lama dinantikan, menghadirkan sistem mod modular, serta menambahkan dukungan bahasa antarmuka (Bahasa Indonesia & English) yang konsisten di semua tab.

### 🌟 Fitur Baru (New Features)

- **Quick Sticky Notes (Catatan Cepat):**
  - Tab baru *"Catatan"* (*Notes*) di notch dengan layout 2-kolom yang ringkas: daftar memo di kiri dan area baca/tulis di kanan.
  - 5 palet warna pastel yang lembut (Kuning, Biru, Hijau, Pink, Ungu) untuk mengelompokkan catatan Anda secara visual.
  - Fitur **Pop to Desktop**: klik tombol `⧉ Pop` untuk melepas catatan menjadi jendela melayang independen di desktop dengan pin *always-on-top*.
  - Ekspor sekali klik: simpan catatan langsung ke format Markdown (`.md`) atau Plain Text (`.txt`), atau salin langsung seluruh isinya ke clipboard.
  - Mekanisme *auto-save* instan tanpa perlu repot menekan tombol simpan manual.
- **Sistem Mod & Ekstensi Modular:**
  - Kategori baru *"Mod & Ekstensi"* di menu Pengaturan. Mod dikemas secara modular berbasis manifest (`.mod.json`) tanpa service latar belakang liar yang membebani CPU/RAM ketika fitur tidak diaktifkan.
- **Dukungan Dua Bahasa Lengkap (Bilingual ID / EN):**
  - Opsi pilihan bahasa di menu Pengaturan: **Bahasa Indonesia** dan **English**.
  - Seluruh elemen antarmuka telah dilokalisasi secara utuh: judul tab atas, kalender (nama hari & bulan), submode reminder, status fokus/hidrasi/peregangan/sholat, tombol aksi, menu dropdown, kartu libur nasional, hingga pop-up toast notifikasi.
- **Perilaku Stage 1 Cerdas (Smart Auto-revert):**
  - Notch kini memiliki intuisi konteks: ketika Anda sedang mendengarkan musik dengan mode lirik aktif atau saat timer fokus sedang berjalan, notch akan tetap setia menampilkan status tersebut di Stage 1.
  - Untuk tab lainnya (catatan, riwayat clipboard yang baru disalin, pengingat hidrasi, peregangan, atau sholat), notch akan menampilkan informasinya selama 5 detik, kemudian bertransisi kembali secara anggun (*smooth morph*) ke tampilan `Tanggal | Jam`.

### ⚡ Peningkatan & Kenyamanan UI/UX (Improvements & Polish)

- **Pintasan `Ctrl+A` (Select All):** Kini Anda dapat memblok seluruh teks sekaligus di kotak judul catatan, kotak isi catatan, maupun kolom input teks lainnya tanpa suara *beep* Windows.
- **Navigasi Roda Mouse (Mouse Wheel Scroll):** Gulirkan roda mouse untuk berpindah bulan secara cepat di Kalender, atau berpindah halaman di daftar riwayat Clipboard.
- **Tombol "Ikuti" (Follow Lyrics):** Saat Anda menggulir teks lirik musik secara manual, tombol *"Ikuti"* akan muncul secara dinamis agar Anda bisa langsung melompat kembali ke baris lagu yang sedang dinyanyikan.
- **Mode Minimal Disempurnakan:** Tampilan bar kecil (*minimal mode*) kini berupa kapsul hitam ramping yang solid tanpa glitch transparan atau outline artifisial.
- **Pembersihan Efek Frosted Glass:** Menghilangkan efek frosted glass sintetis agar tampilan notch tetap setia pada estetika minimalis, bersih, dan berbobot ringan.
- **Hover Debounce 140 ms:** Menambahkan jeda mikro saat kursor mouse menyentuh notch agar bilah tidak terpicu melebar tanpa sengaja saat kursor sekadar melintas.
- **Penyesuaian Label Sholat:** Menampilkan teks *"Tepat waktu"* (atau *"On time"*) yang lebih natural ketika lead time notifikasi bernilai 0 menit.
- **Lompat ke Libur Terdekat:** Mengklik kartu *"Libur Nasional Berikutnya"* di kalender kini langsung membawa tampilan kalender tepat ke tanggal libur yang bersangkutan.
- **Fokus Keyboard & Aksesibilitas:** Seluruh kontrol tombol dan kartu interaktif kini memiliki penanda *focus ring* dan mendukung penekanan melalui tombol `Space` atau `Enter`.

### 🐛 Perbaikan Bug (Bug Fixes)

- **Isolasi Draf Catatan:** Memperbaiki bug di mana catatan yang baru dibuat setelah penghapusan catatan lama masih menampilkan isi teks sebelumnya.
- **DirectWrite Word Wrapping:** Memperbaiki penanganan cache perender teks multiline agar teks panjang melakukan pembungkusan kata (*word-wrap*) secara rapi dan tidak meluap keluar layar.
- **Pencegahan Kebocoran Memori & Handle:** Memperbaiki pelepasan handle GDI dan alokasi memori internal ICU pada pengurai lirik dan pengelola clipboard.
- **Perlindungan Privasi Clipboard:** Mengabaikan secara otomatis format clipboard sensitif (`ClipboardHistoryIgnore` / `ExcludeClipboardContentFromMonitorProcessing`) dari aplikasi pengelola kata sandi (*Password Manager*).
- **Transisi State Dropdown:** Memastikan menu pop-up pengaturan dan reminder tertutup bersih saat notch kembali ke mode bar kecil atau mode istirahat.
- **Penanganan Notifikasi Prioritas:** Memperbaiki urutan klasifikasi toast agar notifikasi selesainya sesi fokus mendapatkan prioritas visual yang tepat.

---

## [0.0.1 alpha] — 26 Oktober 2026

- Paket perdana dengan nama **mynotch**, executable `mynotch.exe`, dan distribusi portable Windows x64.
- Integrasi kontrol media musik, lirik sinkron, visualizer spektrum audio WASAPI.
- Kalender bulanan dengan penanda resmi Hari Libur Nasional Indonesia 2026.
- Empat pilar pengingat harian: Fokus (Pomodoro), Hidrasi, Peregangan tubuh, dan Jadwal Sholat berbasis data myQuran (WIB).
- Riwayat teks Clipboard opsional dengan penyimpanan lokal.
- Navigasi Pengaturan terstruktur di sisi kiri dengan kontrol di sisi kanan.
- Fallback Direct3D software WARP otomatis jika inisialisasi hardware Direct2D memerlukan dukungan cadangan.
