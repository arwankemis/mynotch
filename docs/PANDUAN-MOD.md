# Panduan Membuat dan Memasang Mod MyNotch

MyNotch mendukung sistem mod eksternal. Pengguna dan pengembang komunitas dapat menambahkan halaman mod baru tanpa perlu mengompilasi ulang aplikasi utama. Secara default mod komunitas dikelola dari **Pengaturan → Mod & Ekstensi** dan tidak memenuhi bilah tab. Setiap mod punya toggle **Tab** di panel detail: aktifkan bila ingin halamannya muncul sebagai pill tab di Stage 2, seperti mod bawaan Cuaca.

Mod dapat dibuat dalam dua format, yaitu deklaratif berbasis JSON atau dinamis berbasis skrip Lua.

## Struktur Folder Mod

Semua mod eksternal disimpan di dalam direktori `data/mods/` di sebelah file eksekusi `mynotch.exe`.

Setiap mod folder memiliki berkas utama `mod.json`:

```text
data/
  mods/
    kurs_usd/
      mod.json
    counter_lua/
      mod.json
      main.lua
```

Anda juga dapat meletakkan mod berkas tunggal berakhiran `.mod.json` langsung di dalam folder `data/mods/`, misalnya `data/mods/ringkasan.mod.json`.

## Jenis Mod

### 1. Mod Deklaratif (JSON)

Mod deklaratif cocok untuk menampilkan kumpulan kartu informasi statis, tautan cepat, atau ringkasan metrik. Mod ini tidak memerlukan kode pemrograman.

Contoh `mod.json` deklaratif:

```json
{
  "id": "kurs_usd",
  "name": "Kurs Valuta",
  "tab_name": "Kurs",
  "description": "Pantau kurs mata uang asing dan informasi pasar.",
  "version": "1.0.0",
  "author": "Komunitas",
  "icon": "globe",
  "mod_type": "declarative",
  "cards": [
    {
      "title": "USD ke IDR",
      "value": "Rp 16.280",
      "subtext": "Bank Indonesia, Kurs Tengah"
    },
    {
      "title": "EUR ke IDR",
      "value": "Rp 17.650",
      "subtext": "Pasar Spot"
    }
  ],
  "buttons": [
    {
      "label": "Buka Situs BI",
      "action": "open_url",
      "target": "https://www.bi.go.id"
    },
    {
      "label": "Salin Kurs",
      "action": "copy",
      "target": "Kurs USD saat ini: Rp 16.280"
    },
    {
      "label": "Notifikasi",
      "action": "toast",
      "target": "Pasar valuta terpantau stabil."
    }
  ]
}
```

Bidang tombol mendukung tiga jenis aksi bawaan:
- `open_url`: membuka alamat situs web di peramban standar sistem operasi.
- `copy`: menyalin teks target ke clipboard Windows.
- `toast`: memunculkan banner notifikasi cepat di notch.

> `tab_name` menjadi label pill tab bila toggle **Tab** diaktifkan. Selama toggle mati, mod hanya dibuka lewat tombol Buka dan tidak muncul di bilah tab.

### 2. Mod Skrip Lua

Mod skrip memberikan fleksibilitas logika dinamis dan penanganan tombol interaktif. Mod Lua dijalankan di dalam mesin virtual Lua 5.4 pure Rust yang terisolasi (sandbox). Skrip tidak memiliki akses langsung ke sistem berkas atau perintah shell sistem operasi.

Struktur berkas `mod.json`:

```json
{
  "id": "kontrol_cepat",
  "name": "Pusat Kontrol",
  "tab_name": "Kontrol",
  "description": "Contoh halaman interaktif menggunakan skrip Lua.",
  "version": "1.0.0",
  "author": "Nama Anda",
  "icon": "terminal",
  "mod_type": "lua",
  "entry": "main.lua"
}
```

Contoh skrip `main.lua`:

```lua
-- Kartu metrik
ui.card("Status Modul", "Aktif", "Lingkungan sandbox aman")
ui.card("Sesi Kerja", "45 Menit", "Target berikutnya")

-- Tombol interaktif
if ui.button("Buka Repositori", "btn_repo") then
    system.open_url("https://github.com/arwankemis/mynotch")
    system.toast("Membuka repositori GitHub...")
end

if ui.button("Salin Teks", "btn_copy") then
    system.copy("Teks ini disalin dari mod MyNotch.")
    system.toast("Teks berhasil disalin ke clipboard.")
end
```

#### API Modul Lua yang Tersedia

Antarmuka UI:
- `ui.card(title, value, subtext)`: membuat kartu ringkasan visual.
- `ui.button(label, id)`: membuat tombol dan mengembalikan nilai boolean true saat tombol diklik oleh pengguna pada frame interaksi.
- `ui.text(text, x, y)`: meletakkan teks bebas pada koordinat bidang gambar.
- `ui.headline(label, value, delta, meta, badge)`: band ringkasan di atas daftar, misalnya nilai indeks. `meta` dan `badge` opsional. Tersedia di MyNotch `0.0.1.3` ke atas.
- `ui.ranked(side, title, meta, rows)`: satu lane daftar peringkat. `side` berisi `"left"` atau `"right"`. `rows` adalah array berisi `ticker`, `name`, `price`, dan `change` (angka). Maksimal 5 baris per lane. Tersedia di MyNotch `0.0.1.3` ke atas.

Contoh lane peringkat:

```lua
if ui.headline and ui.ranked then
    ui.headline("IHSG · IDX Composite", "7.648,32", "+32,14 (+0,42%)", "Sesi 2", "CONTOH DATA")
    ui.ranked("left", "Top 5 Gainer", "5 emiten · +8,32%", {
        { ticker = "BREN", name = "Barito Renewables", price = "6.475", change = 12.55 },
        { ticker = "CUAN", name = "Petrindo Jaya Kreasi", price = "3.120", change = 9.86 }
    })
    ui.ranked("right", "Top 5 Loser", "5 emiten · -4,90%", {
        { ticker = "GOTO", name = "GoTo Gojek Tokopedia", price = "68", change = -6.85 }
    })
end
```

Catatan: bila `ui.headline`/`ui.ranked` dipakai, kartu dan tombol pada halaman yang sama tidak digambar. Selalu cek `if ui.headline and ui.ranked` agar mod tetap jalan di build lama dan bisa jatuh ke `ui.card`.

Antarmuka Sistem:
- `system.toast(message)`: menampilkan pesan notifikasi di notch.
- `system.open_url(url)`: membuka tautan di browser.
- `system.copy(text)`: menyalin teks ke papan klip Windows.
- `system.config`: tabel dari `config.json` di folder mod. Kosong bila berkas tidak ada.
- `system.http_get(url)`: mengembalikan `body, status`. Status berisi `ok`, `pending`, `error`, atau `blocked`. Fetch berjalan di background dengan cache; panggil lagi saat status `pending`.
- `system.json_decode(text)`: mengubah teks JSON menjadi tabel Lua, atau `nil` bila tidak valid.

#### Sumber data dan config.json

Mod yang membutuhkan jaringan menyimpan endpoint di `config.json` di folder mod:

```json
{
  "refresh_seconds": 60,
  "endpoints": [
    "https://contoh.test/data"
  ]
}
```

Aturan penting:
- `system.http_get` hanya berfungsi setelah toggle **Net** di panel detail mod dinyalakan. Default mati.
- Hanya HTTPS, tanpa cookie/redirect, respons dibatasi 8 MiB, dan satu request berjalan pada satu waktu.
- Cache dianggap segar selama `refresh_seconds` (15–600 detik, default 60).
- Mod boleh memakai satu `endpoint` (string) atau beberapa `endpoints` (array). Beberapa endpoint diambil berurutan saat dibuka.
- Endpoint bisa diganti tanpa menyentuh kode. Bentuk respons tetap harus cocok dengan parser di `main.lua`; untuk API dengan bentuk berbeda, sesuaikan fungsi `parse_bodies` di skrip mod.

Sandbox Lua hanya memuat sebagian pustaka standar: `string.sub/len/reverse/lower/upper`, `table.pack/unpack`, dan `math`. Tidak ada `string.format`, `string.gsub`, `table.sort`, `os`, atau `io`. Tulis format angka dan pengurutan secara manual seperti pada mod IHSG.

Batas instruksi: skrip dilindungi pembatas fuel untuk mencegah perulangan tak terbatas yang dapat memperlambat antarmuka.

## Mengemas Mod ke Format ZIP

Untuk membagikan mod kepada pengguna lain:

1. Pilih berkas `mod.json` beserta berkas pendukung seperti `main.lua` dan `config.json`.
2. Klik kanan lalu pilih kompresi ke format `.zip`.
3. Berkas arsip siap didistribusikan.

## Memasang Mod di MyNotch

Terdapat dua cara mudah untuk memasang mod:

1. Melalui Menu Pengaturan:
   Buka Pengaturan MyNotch, pilih kategori Mod & Ekstensi, lalu klik tombol "+ Impor Mod". Pilih berkas `.zip` atau `.json` yang ingin dipasang. MyNotch akan mengekstrak berkas, memvalidasi manifest, dan menambahkan mod ke daftar.

2. Secara Manual lewat File Explorer:
   Pada kategori Mod & Ekstensi, klik tombol "Folder Mod". Windows Explorer akan terbuka di folder `data/mods/`. Anda cukup menyalin folder mod ke dalam direktori tersebut; daftar akan dipindai ulang saat kategori Mod dibuka.

Untuk membuka mod, pilih mod di daftar kiri lalu klik tombol **Buka** pada panel detail. Halaman mod tampil penuh di Stage 2 dengan tombol **‹ Kembali** untuk kembali ke Pengaturan.

Ingin halaman mod muncul sebagai tab? Aktifkan toggle **Tab** di panel detail. Setelah itu mod mendapat pill tab di bilah atas dan bisa dibuka langsung dari Stage 2. Mematikan toggle menyembunyikan pill tanpa menghapus posisi tab; mengaktifkannya lagi mengembalikan posisi yang sama. Pengaturan ini tersimpan di `settings.json` (`mod_tabs`).

Mod yang memanggil `system.http_get` juga memerlukan toggle **Net** di panel detail. Default mati; tanpa izin ini, mod hanya menampilkan fallback lokal. Izin disimpan di `settings.json` (`network_mods`).

Untuk menghapus mod komunitas, gunakan tombol **Hapus**. Tabnya (jika pernah diaktifkan) ikut dibersihkan. Tombol **Pasang/Lepas** hanya berlaku untuk mod bawaan (Catatan dan Cuaca) karena keduanya memiliki tab asli di bilah atas.

## Batasan jaringan dan aksi pada v0.0.2

Permintaan jaringan memerlukan izin mod. Tujuan jaringan privat dan alamat cadangan diblokir secara default. `system.open_url` hanya menerima HTTPS; membuka browser dan menyalin teks melalui aksi mod harus dipicu klik tombol pengguna. Jangan menaruh token atau konfigurasi akun dalam paket mod yang dibagikan.
