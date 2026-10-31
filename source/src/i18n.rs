pub const LANGUAGES: [(&str, &str); 2] = [
    ("id", "Bahasa Indonesia"),
    ("en", "English"),
];

pub fn language_name(code: &str) -> &'static str {
    match code {
        "en" => "English",
        _ => "Bahasa Indonesia",
    }
}

pub fn tab_name<'a>(tab: &'a str, lang: &str) -> &'a str {
    let en = lang == "en";
    match tab {
        "Musik" => if en { "Music" } else { "Musik" },
        "Kalender" => if en { "Calendar" } else { "Kalender" },
        "Reminder" => if en { "Reminders" } else { "Reminder" },
        "Clipboard" => "Clipboard",
        "Catatan" => if en { "Notes" } else { "Catatan" },
        "Tugas" => if en { "Tasks" } else { "Tugas" },
        "Cuaca" => if en { "Weather" } else { "Cuaca" },

        "Daya" => if en { "Power" } else { "Daya" },
        _ => tab,
    }
}

pub fn month_name(month: u32, lang: &str) -> &'static str {
    let en = lang == "en";
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => if en { "May" } else { "Mei" },
        6 => "Jun",
        7 => "Jul",
        8 => if en { "Aug" } else { "Ags" },
        9 => "Sep",
        10 => if en { "Oct" } else { "Okt" },
        11 => "Nov",
        _ => if en { "Dec" } else { "Des" },
    }
}

pub fn full_month_name(month: u32, lang: &str) -> &'static str {
    let en = lang == "en";
    match month {
        1 => if en { "January" } else { "Januari" },
        2 => if en { "February" } else { "Februari" },
        3 => if en { "March" } else { "Maret" },
        4 => if en { "April" } else { "April" },
        5 => if en { "May" } else { "Mei" },
        6 => if en { "June" } else { "Juni" },
        7 => if en { "July" } else { "Juli" },
        8 => if en { "August" } else { "Agustus" },
        9 => if en { "September" } else { "September" },
        10 => if en { "October" } else { "Oktober" },
        11 => if en { "November" } else { "November" },
        _ => if en { "December" } else { "Desember" },
    }
}

pub fn short_day_name(idx: usize, lang: &str) -> &'static str {
    let en = lang == "en";
    match idx % 7 {
        0 => if en { "Mon" } else { "Sen" },
        1 => if en { "Tue" } else { "Sel" },
        2 => if en { "Wed" } else { "Rab" },
        3 => if en { "Thu" } else { "Kam" },
        4 => if en { "Fri" } else { "Jum" },
        5 => if en { "Sat" } else { "Sab" },
        _ => if en { "Sun" } else { "Min" },
    }
}

pub fn day_name_upper(idx: usize, lang: &str) -> &'static str {
    let en = lang == "en";
    match idx % 7 {
        0 => if en { "MONDAY" } else { "SENIN" },
        1 => if en { "TUESDAY" } else { "SELASA" },
        2 => if en { "WEDNESDAY" } else { "RABU" },
        3 => if en { "THURSDAY" } else { "KAMIS" },
        4 => if en { "FRIDAY" } else { "JUMAT" },
        5 => if en { "SATURDAY" } else { "SABTU" },
        _ => if en { "SUNDAY" } else { "MINGGU" },
    }
}

pub fn day_name_capitalized(idx: usize, lang: &str) -> &'static str {
    let en = lang == "en";
    match idx % 7 {
        0 => if en { "Monday" } else { "Senin" },
        1 => if en { "Tuesday" } else { "Selasa" },
        2 => if en { "Wednesday" } else { "Rabu" },
        3 => if en { "Thursday" } else { "Kamis" },
        4 => if en { "Friday" } else { "Jumat" },
        5 => if en { "Saturday" } else { "Sabtu" },
        _ => if en { "Sunday" } else { "Minggu" },
    }
}

pub fn reminder_name(idx: usize, lang: &str) -> &'static str {
    let en = lang == "en";
    match idx % 4 {
        0 => if en { "Focus" } else { "Fokus" },
        1 => if en { "Hydration" } else { "Hidrasi" },
        2 => if en { "Stretch" } else { "Peregangan" },
        _ => if en { "Prayer" } else { "Sholat" },
    }
}

pub fn prayer_name(idx: usize, lang: &str) -> &'static str {
    let en = lang == "en";
    match idx % 5 {
        0 => if en { "Fajr" } else { "Subuh" },
        1 => if en { "Dhuhr" } else { "Zuhur" },
        2 => if en { "Asr" } else { "Asar" },
        3 => if en { "Maghrib" } else { "Magrib" },
        _ => if en { "Isha" } else { "Isya" },
    }
}

pub fn clip_meta(chars: usize, lines: usize, lang: &str) -> String {
    if lang == "en" {
        format!("{chars} chars · {lines} lines")
    } else {
        format!("{chars} kar · {lines} baris")
    }
}

pub fn more_lines(count: usize, lang: &str) -> String {
    if lang == "en" {
        format!("+ {count} more lines...")
    } else {
        format!("+ {count} baris lainnya...")
    }
}

pub fn notes_count(total: usize, lang: &str) -> String {
    if lang == "en" {
        format!("{total} / 20 notes")
    } else {
        format!("{total} / 20 catatan")
    }
}

pub fn focus_summary(minutes: u32, lang: &str) -> String {
    if lang == "en" {
        format!("{minutes} min session")
    } else {
        format!("Sesi {minutes} menit")
    }
}

pub fn move_summary(minutes: u32, lang: &str) -> String {
    if lang == "en" {
        format!("Every {minutes} min")
    } else {
        format!("Setiap {minutes} menit")
    }
}

pub fn move_desc(stretch_secs: u32, interval_mins: u32, lang: &str) -> String {
    let dur = stretch_secs / 60;
    if lang == "en" {
        format!("{dur} min break every {interval_mins} min.")
    } else {
        format!("Jeda {dur} menit setiap {interval_mins} menit.")
    }
}

pub fn t<'a>(key: &'a str, lang: &str) -> &'a str {
    let en = lang == "en";
    match key {
        // Settings Common
        "settings.title" => if en { "Settings" } else { "Pengaturan" },
        "settings.auto_saved" => if en { "Changes saved automatically" } else { "Perubahan tersimpan otomatis" },
        "settings.back" => if en { "Back" } else { "Kembali" },

        // Settings Categories
        "cat.general" => if en { "General" } else { "Umum" },
        "cat.general.desc" => if en { "Portable Windows notch settings." } else { "Pengaturan notch portable Windows." },
        "cat.appearance" => if en { "Appearance" } else { "Tampilan" },
        "cat.appearance.desc" => if en { "Position and visual comfort." } else { "Posisi dan kenyamanan tampilan." },
        "cat.tabs" => if en { "Tabs & navigation" } else { "Tab & navigasi" },
        "cat.tabs.desc" => if en { "Reorder tabs; keep at least one active." } else { "Atur urutan; sisakan satu tab aktif." },
        "cat.audio" => if en { "Music" } else { "Musik" },
        "cat.audio.desc" => if en { "Player, lyrics, and audio visualization." } else { "Pemutar, lirik, dan visualisasi audio." },
        "cat.reminders" => if en { "Reminders" } else { "Reminder" },
        "cat.reminders.desc" => if en { "Daily reminder and health settings." } else { "Buka pengaturan pengingat harian." },
        "cat.mods" => if en { "Mods & Extensions" } else { "Mod & Ekstensi" },
        "cat.mods.desc" => if en { "Install, open, or remove community mods." } else { "Pasang, buka, atau kelola mod komunitas." },
        "cat.update" => if en { "Updates" } else { "Pembaruan" },
        "cat.update.desc" => if en { "Check and download the latest mynotch releases." } else { "Periksa dan unduh rilis terbaru mynotch." },

        // Settings Update
        "update.current" => if en { "Current Version" } else { "Versi Saat Ini" },
        "update.current_desc" => if en { "Installed release on this machine." } else { "Rilis yang terpasang di komputer ini." },
        "update.check" => if en { "Check for Updates" } else { "Periksa Pembaruan" },
        "update.check_again" => if en { "Check Again" } else { "Cek Ulang" },
        "update.checking" => if en { "Connecting to GitHub Releases..." } else { "Menghubungi GitHub Releases..." },
        "update.up_to_date" => if en { "mynotch is up to date" } else { "mynotch sudah versi terbaru" },
        "update.up_to_date_sub" => if en { "You are using the latest official build." } else { "Anda sedang menggunakan versi resmi terbaru." },
        "update.checked_at" => if en { "Last checked" } else { "Terakhir diperiksa" },
        "update.available" => if en { "New Release Available!" } else { "Versi Baru Tersedia!" },
        "update.download_zip" => if en { "Download Update (ZIP)" } else { "Unduh Update (ZIP)" },
        "update.release_notes" => if en { "Release Notes" } else { "Catatan Rilis" },
        "update.repo" => if en { "Official Repository" } else { "Repositori Resmi" },
        "update.repo_desc" => if en { "github.com/arwankemis/mynotch" } else { "github.com/arwankemis/mynotch" },
        "update.open_repo" => if en { "Open GitHub" } else { "Buka GitHub" },
        "update.error" => if en { "Failed to check updates" } else { "Gagal memeriksa pembaruan" },
        "update.retry" => if en { "Retry" } else { "Coba Lagi" },
        "update.idle_hint" => if en { "Click 'Check for Updates' to fetch the latest release from GitHub." } else { "Klik 'Periksa Pembaruan' untuk mengecek rilis terbaru dari GitHub." },
        "update.perbarui" => if en { "Update" } else { "Perbarui" },
        "update.downloading" => if en { "Downloading update..." } else { "Mengunduh pembaruan..." },
        "update.verifying" => if en { "Verifying update..." } else { "Memverifikasi pembaruan..." },
        "update.staging" => if en { "Preparing update..." } else { "Menyiapkan pembaruan..." },
        "update.ready" => if en { "Update ready" } else { "Pembaruan siap" },
        "update.restart_desc" => if en { "Restart mynotch to apply" } else { "Mulai ulang mynotch untuk menerapkan" },
        "update.restart_now" => if en { "Restart now" } else { "Mulai ulang sekarang" },
        "update.restart_later" => if en { "Later" } else { "Nanti" },
        "update.restarting" => if en { "Restarting mynotch..." } else { "Memulai ulang mynotch..." },
        "update.updated_to" => if en { "Updated to" } else { "Berhasil diperbarui ke" },
        "update.rolled_back" => if en { "Update failed to apply. Previous version restored." } else { "Pembaruan gagal diterapkan. Versi sebelumnya dipulihkan." },
        "update.not_installed" => if en { "Update not installed" } else { "Pembaruan belum dipasang" },
        "update.no_package" => if en { "No update package for this build" } else { "Paket untuk build ini belum tersedia" },

        // Settings General
        "general.language" => if en { "Language" } else { "Bahasa" },
        "general.language.desc" => if en { "Select application interface language." } else { "Pilih bahasa antarmuka aplikasi." },
        "general.fullscreen" => if en { "Hide in fullscreen" } else { "Sembunyikan saat layar penuh" },
        "general.fullscreen.desc" => if en { "When gaming or watching videos." } else { "Saat bermain game atau menonton video." },
        "general.minimal" => if en { "Minimal bar mode" } else { "Mode bar kecil" },
        "general.minimal.desc" => if en { "Use a sleek mini bar when notch is collapsed." } else { "Gunakan bar kecil saat notch ditutup." },
        "general.default_tab" => if en { "Startup tab" } else { "Tab saat dibuka" },
        "general.default_tab.desc" => if en { "Initial tab when application starts." } else { "Halaman awal ketika aplikasi berjalan." },
        "general.startup" => if en { "Run at Windows startup" } else { "Jalankan saat startup" },
        "general.startup.desc" => if en { "Automatically launch mynotch on login." } else { "Otomatis jalankan mynotch saat masuk Windows." },
        "general.privacy" => if en { "Privacy & Indicators" } else { "Privasi & Indikator" },
        "general.privacy.desc" => if en { "Configure microphone and camera privacy indicators." } else { "Pengaturan pemantauan dan indikator privasi." },
        "general.privacy_open" => if en { "Configure" } else { "Atur" },

        // Privacy
        "privacy.title" => if en { "Privacy" } else { "Privasi" },
        "privacy.subtitle" => if en { "Microphone, camera, and display usage status." } else { "Status penggunaan mikrofon, kamera, dan tampilan layar." },
        "privacy.mic_title" => if en { "Microphone" } else { "Mikrofon" },
        "privacy.camera_title" => if en { "Camera" } else { "Kamera" },
        "privacy.share_title" => if en { "Screen Sharing" } else { "Berbagi Layar" },
        "privacy.record_title" => if en { "Screen Recording" } else { "Perekaman Layar" },
        "privacy.mic_in_use" => if en { "Microphone in use" } else { "Mikrofon sedang digunakan" },
        "privacy.cam_in_use" => if en { "Camera in use" } else { "Kamera sedang digunakan" },
        "privacy.cam_detected" => if en { "Camera activity detected" } else { "Aktivitas kamera terdeteksi" },
        "privacy.screen_active" => if en { "Screen capture active" } else { "Capture layar aktif" },
        "privacy.share_active" => if en { "Screen sharing active" } else { "Layar sedang dibagikan" },
        "privacy.record_active" => if en { "Screen recording active" } else { "Perekaman layar aktif" },
        "privacy.no_activity" => if en { "No activity detected" } else { "Tidak ada aktivitas terdeteksi" },
        "privacy.status_unknown" => if en { "Status unknown" } else { "Status belum diketahui" },
        "privacy.unavailable" => if en { "Monitoring unavailable" } else { "Belum dapat dipantau" },
        "privacy.limited" => if en { "Limited monitoring" } else { "Pemantauan terbatas" },
        "privacy.app_unknown" => if en { "App identity unavailable" } else { "Identitas aplikasi tidak tersedia" },
        "privacy.since_startup" => if en { "Observed since startup" } else { "Terpantau sejak dibuka" },
        "privacy.open_settings" => if en { "Open Windows settings" } else { "Buka pengaturan Windows" },
        "privacy.settings_title" => if en { "Privacy" } else { "Privasi" },
        "privacy.settings_desc" => if en { "Privacy and device monitoring settings." } else { "Pengaturan privasi dan pemantauan perangkat." },
        "privacy.enabled" => if en { "Enable monitoring" } else { "Aktifkan pemantauan" },
        "privacy.enabled_desc" => if en { "Monitor Windows microphone and camera usage." } else { "Pantau penggunaan mikrofon dan kamera Windows." },
        "privacy.fullscreen" => if en { "Show in fullscreen" } else { "Tampilkan saat fullscreen" },
        "privacy.fullscreen_desc" => if en { "Display indicator when fullscreen app is active." } else { "Tampilkan indikator saat aplikasi fullscreen aktif." },
        "privacy.app_names" => if en { "Show app names" } else { "Tampilkan nama aplikasi" },
        "privacy.app_names_desc" => if en { "Display names of applications currently using devices." } else { "Tampilkan nama aplikasi yang sedang menggunakan perangkat." },
        "privacy.experimental_note" => if en { "Camera monitoring uses experimental system source (ConsentStore)." } else { "Pemantauan kamera menggunakan sumber sistem eksperimental (ConsentStore)." },
        "privacy.mic_note" => if en { "Microphone monitoring uses Windows CoreAudio active audio sessions." } else { "Pemantauan mikrofon menggunakan sesi audio aktif Windows CoreAudio." },
        "privacy.coverage_footer" => if en { "Windows does not provide universal APIs for camera & screen monitoring; camera status is read from system registry (experimental)." } else { "Windows tidak menyediakan API universal untuk pemantauan kamera & layar; status kamera bersumber dari registry sistem (eksperimental)." },

        // Settings Appearance
        "appearance.position" => if en { "Notch position" } else { "Posisi notch" },
        "appearance.position.desc" => if en { "Select screen edge for notch." } else { "Pilih tepi layar untuk notch." },
        "appearance.top" => if en { "Top" } else { "Atas" },
        "appearance.bottom" => if en { "Bottom" } else { "Bawah" },
        "appearance.scale" => if en { "Interface scale" } else { "Ukuran antarmuka" },
        "appearance.scale.desc" => if en { "Scale text and controls proportionally." } else { "Teks dan kontrol diperbesar bersama." },
        "appearance.gap" => if en { "Edge gap" } else { "Jarak dari tepi" },
        "appearance.gap.desc" => if en { "0 px keeps notch flush against screen." } else { "0 px membuat notch menempel layar." },
        "appearance.opacity" => if en { "Background opacity" } else { "Kepekatan latar" },
        "appearance.opacity.desc" => if en { "Higher value makes background denser." } else { "Nilai tinggi membuat latar lebih pekat." },
        "appearance.material" => if en { "Surface and color" } else { "Permukaan dan warna" },
        "appearance.material.desc" => if en { "Solid or glass; dark or light." } else { "Solid/kaca; gelap/terang." },
        "material.solid" => "Solid",
        "material.lite" => "Glass Lite",
        "material.adaptive" => "Glass Adaptive",
        "color.dark" => if en { "Dark" } else { "Gelap" },
        "color.light" => if en { "Light" } else { "Terang" },
        "material.fallback.reduced" => if en { "Solid: Windows transparency is off" } else { "Solid: transparansi Windows mati" },
        "material.fallback.contrast" => if en { "Solid: High Contrast is on" } else { "Solid: Kontras Tinggi aktif" },
        "material.fallback.software" => if en { "Solid: software renderer" } else { "Solid: renderer perangkat lunak" },
        "material.fallback.unavailable" => if en { "Solid: backdrop unavailable" } else { "Solid: backdrop tidak tersedia" },

        // Settings Audio
        "audio.visualizer" => if en { "Music visualizer" } else { "Visualizer musik" },
        "audio.visualizer.desc" => if en { "Animated bars responding to audio playback." } else { "Animasi mengikuti audio yang diputar." },
        "audio.glow" => if en { "Cover glow effect" } else { "Efek warna sampul" },
        "audio.glow.desc" => if en { "Use album art color accent on player." } else { "Gunakan warna sampul pada pemutar." },
        "audio.romanize" => if en { "Romanize CJK lyrics" } else { "Romanisasi lirik CJK" },
        "audio.romanize.desc" => if en { "Convert Japanese/Korean/Chinese to Latin." } else { "Ubah aksara Jepang/Korea/Mandarin ke Latin." },
        "audio.player_lyrics" => if en { "Player and lyrics" } else { "Pemutar dan lirik" },
        "audio.player_lyrics.desc" => if en { "View playback and synchronized lyrics." } else { "Kontrol musik yang sedang diputar." },
        "audio.open_music" => if en { "Open Music" } else { "Buka Musik" },

        // Settings Reminders
        "reminders.disabled" => if en { "Reminders tab is currently disabled." } else { "Tab Reminder sedang dimatikan." },
        "reminders.enable" => if en { "Enable" } else { "Aktifkan" },
        "reminders.configure" => if en { "Configure" } else { "Atur" },
        "reminders.active" => if en { "Active" } else { "Aktif" },
        "reminders.inactive" => if en { "Off" } else { "Mati" },
        "reminders.focus" => if en { "Focus" } else { "Fokus" },
        "reminders.hydrate" => if en { "Hydration" } else { "Hidrasi" },
        "reminders.stretch" => if en { "Stretch" } else { "Peregangan" },
        "reminders.prayer" => if en { "Prayer" } else { "Sholat" },

        // Stage 1 & Tab info
        "stage1.no_music" => if en { "No music playing" } else { "Belum ada musik" },
        "stage1.clips" => if en { "clips" } else { "klip" },
        "stage1.notes" => if en { "notes" } else { "catatan" },
        "stage1.new_note" => if en { "New Note" } else { "Catatan Baru" },
        "stage1.stretch" => if en { "Stretch" } else { "Peregangan" },
        "stage1.prayer" => if en { "Prayer" } else { "Sholat" },

        // Shell & Navigation
        "shell.select_module" => if en { "Select module in Settings." } else { "Pilih modul melalui Pengaturan." },

        // Music Player & Lyrics
        "music.no_music" => if en { "No music playing" } else { "Belum ada musik" },
        "music.no_artist" => if en { "Unknown artist" } else { "Tidak ada artis" },
        "music.no_album" => if en { "Unknown single / album" } else { "Single / Album tidak diketahui" },
        "music.single_album" => if en { "Single / Album" } else { "Single / Album" },
        "music.loading_lyrics" => if en { "Loading lyrics…" } else { "Memuat lirik…" },
        "music.no_lyrics" => if en { "Lyrics not available for this song." } else { "Lirik belum tersedia untuk lagu ini." },
        "music.lyrics_error" => if en { "Lyrics failed to load. Check your connection and try again." } else { "Lirik gagal dimuat. Periksa koneksi lalu coba lagi." },
        "music.follow" => if en { "Follow" } else { "Ikuti" },
        "music.open_player_first" => if en { "Open music player first." } else { "Buka pemutar musik terlebih dahulu." },

        // Calendar
        "calendar.national_holiday" => if en { "National holiday" } else { "Libur nasional" },
        "calendar.collective_leave" => if en { "Collective leave" } else { "Cuti bersama" },
        "calendar.footer_2026" => if en { "Indonesia · 2026 holiday data" } else { "Indonesia · data libur 2026" },
        "calendar.footer_other" => if en { "Holiday data for this year is unavailable" } else { "Data libur tahun ini belum tersedia" },
        "calendar.leave_toggle_on" => if en { "☑ Collective leave" } else { "☑ Cuti bersama" },
        "calendar.leave_toggle_off" => if en { "☐ Collective leave" } else { "☐ Cuti bersama" },
        "calendar.no_holiday_data" => if en { "Holiday data unavailable" } else { "Data libur belum tersedia" },
        "calendar.today_regular" => if en { "Today · regular day" } else { "Hari ini · bukan hari libur" },
        "calendar.sunday_weekend" => if en { "Sunday · weekend" } else { "Minggu · akhir pekan" },
        "calendar.regular_day" => if en { "Regular weekday" } else { "Bukan hari libur nasional" },
        "calendar.today_btn" => if en { "Today" } else { "Hari ini" },
        "calendar.next_holiday" => if en { "LIBUR NASIONAL BERIKUTNYA" } else { "LIBUR NASIONAL BERIKUTNYA" },

        // Clipboard
        "clipboard.title" => if en { "CLIP HISTORY" } else { "RIWAYAT KLIP" },
        "clipboard.clear" => if en { "Clear" } else { "Bersihkan" },
        "clipboard.no_history" => if en { "No history yet." } else { "Belum ada riwayat." },
        "clipboard.hint" => if en { "Text you copy (Ctrl+C) in Windows will be automatically saved and shown here." } else { "Teks yang Anda salin (Ctrl+C) di Windows akan otomatis tersimpan dan muncul di sini." },
        "clipboard.plaintext_note" => if en { "History is stored unencrypted in state.json; unpinned items expire after 24 hours, pinned items after 30 days." } else { "Riwayat tersimpan tanpa enkripsi di state.json; item biasa kedaluwarsa 24 jam, yang disematkan 30 hari." },
        "clipboard.empty_text" => if en { "(Empty text)" } else { "(Teks kosong)" },
        "clipboard.empty_title" => if en { "Clipboard History Empty" } else { "Riwayat Clipboard Kosong" },
        "clipboard.empty_desc1" => if en { "Copy text (Ctrl+C) anywhere," } else { "Salin teks (Ctrl+C) di mana saja," },
        "clipboard.empty_desc2" => if en { "Notch will automatically save it here." } else { "Notch akan otomatis menyimpannya di sini." },
        "clipboard.clip" => if en { "Clip" } else { "Klip" },
        "clipboard.recopy" => if en { "📋 Copy Again" } else { "📋 Salin Ulang" },
        "clipboard.delete" => if en { "🗑 Delete" } else { "🗑 Hapus" },

        // Notes (Sticky Notes)
        "notes.header" => if en { "STICKY NOTES" } else { "CATATAN STICKY" },
        "notes.new_btn" => if en { "+ New" } else { "+ Baru" },
        "notes.no_notes" => if en { "No notes yet." } else { "Belum ada catatan." },
        "notes.empty_hint" => if en { "Click '+ New' above to create your first sticky note." } else { "Klik tombol '+ Baru' di atas untuk membuat catatan tempel pertama Anda." },
        "notes.default_title" => if en { "New Note" } else { "Catatan Baru" },
        "notes.empty_snippet" => if en { "(Empty)" } else { "(Kosong)" },
        "notes.empty_desc1" => if en { "Quickly jot down memos or ideas here." } else { "Catat memo atau ide instan di sini." },
        "notes.empty_desc2" => if en { "Can be exported to Markdown (.md) or TXT." } else { "Bisa diexport ke Markdown (.md) atau TXT." },
        "notes.copy" => if en { "📋 Copy" } else { "📋 Salin" },
        "notes.delete" => if en { "🗑 Delete" } else { "🗑 Hapus" },
        "notes.updated" => if en { "Updated" } else { "Diperbarui" },
        "notes.window_title" => if en { "Sticky Note" } else { "Catatan Sticky" },

        // Reminder Common & Focus
        "reminder.remaining" => if en { "remaining" } else { "tersisa" },
        "reminder.narrow" => if en { "Expand the notch to view Reminders." } else { "Perluas notch untuk melihat Reminder." },
        "reminder.break_time" => if en { "Break time" } else { "Waktunya istirahat" },
        "reminder.focusing" => if en { "Focusing" } else { "Sedang fokus" },
        "reminder.paused" => if en { "Session paused" } else { "Sesi dijeda" },
        "reminder.ready" => if en { "Ready to focus" } else { "Siap untuk fokus" },
        "reminder.focus_tip" => if en { "Focus on one task at a time." } else { "Kerjakan satu hal dulu." },
        "reminder.pause" => if en { "Pause" } else { "Jeda" },
        "reminder.resume" => if en { "Resume" } else { "Lanjut" },
        "reminder.start" => if en { "Start" } else { "Mulai" },
        "reminder.focus_duration" => if en { "Focus duration" } else { "Durasi fokus" },
        "reminder.break_duration" => if en { "Break" } else { "Istirahat" },
        "reminder.auto_break" => if en { "Auto-start break" } else { "Mulai istirahat otomatis" },

        // Reminder Hydration
        "reminder.ml_today" => if en { "ml today" } else { "ml hari ini" },
        "reminder.of_target" => if en { "of target" } else { "dari target" },
        "reminder.remind_every" => if en { "Remind every" } else { "Ingatkan setiap" },
        "reminder.daily_target" => if en { "Daily target" } else { "Target harian" },
        "reminder.cup_size" => if en { "Single drink amount" } else { "Takaran sekali minum" },

        // Reminder Stretch
        "reminder.stretching" => if en { "stretching" } else { "peregangan" },
        "reminder.stretching_now" => if en { "Stretching in progress" } else { "Sedang peregangan" },
        "reminder.stretch_tip" => if en { "Roll shoulders, neck, & back." } else { "Gerakkan bahu, leher, & punggung." },
        "reminder.stretch_done" => if en { "Done stretching" } else { "Sudah peregangan" },
        "reminder.move_break" => if en { "move break" } else { "jeda bergerak" },
        "reminder.stand_up" => if en { "Stand up from chair" } else { "Bangun dari kursi" },
        "reminder.start_break" => if en { "Start break" } else { "Mulai jeda" },
        "reminder.done" => if en { "Done" } else { "Sudah" },
        "reminder.break_duration_label" => if en { "Break duration" } else { "Durasi jeda" },
        "reminder.stretch_after_focus" => if en { "Remind when focus session ends" } else { "Pengingat saat sesi fokus selesai" },

        // Reminder Prayer
        "reminder.next" => if en { "Next" } else { "Berikutnya" },
        "reminder.prayer_title" => if en { "Prayer reminder" } else { "Pengingat sholat" },
        "reminder.today_schedule" => if en { "Today's schedule" } else { "Jadwal hari ini" },
        "reminder.loading_prayers" => if en { "Loading prayer times…" } else { "Memuat jadwal sholat…" },
        "reminder.no_schedule" => if en { "Schedule unavailable" } else { "Jadwal belum tersedia" },
        "reminder.city" => if en { "City" } else { "Kota" },
        "reminder.prayer_alert" => if en { "Reminder" } else { "Pengingat" },
        "reminder.on_time" => if en { "On time" } else { "Tepat waktu" },
        "reminder.min_before" => if en { "min before" } else { "menit sebelum" },
        "reminder.sound_alert" => if en { "Sound reminder" } else { "Suara pengingat" },
        "reminder.load_failed" => if en { "Failed to load · click to retry" } else { "Gagal memuat · klik untuk mencoba lagi" },
        "reminder.five_times" => if en { "5 daily prayers" } else { "Lima waktu" },

        // Notices & Feedback
        "notice.pinned" => if en { "Notch pinned" } else { "Notch disematkan (pin)" },
        "notice.unpinned" => if en { "Notch unpinned" } else { "Sematan notch dilepas" },
        "notice.mod_installed" => if en { "Mod installed. Open it from Settings → Mod." } else { "Mod berhasil dipasang. Buka dari Pengaturan → Mod." },
        "notice.mod_uninstalled" => if en { "Mod uninstalled." } else { "Mod berhasil dilepas." },
        "notice.mod_deleted" => if en { "Mod deleted." } else { "Mod berhasil dihapus." },
        "mods.delete_btn" => if en { "Delete" } else { "Hapus" },
        "mods.import_btn" => if en { "+ Import Mod" } else { "+ Impor Mod" },
        "mods.folder_btn" => if en { "Mods Folder" } else { "Folder Mod" },
        "mods.open_btn" => if en { "Open" } else { "Buka" },
        "mods.install_btn" => if en { "Install" } else { "Pasang" },
        "mods.uninstall_btn" => if en { "Uninstall" } else { "Lepas" },
        "mods.type_builtin" => if en { "Built-in" } else { "Bawaan" },
        "mods.type_json" => "JSON",
        "mods.type_lua" => "Lua",
        "mods.tab_toggle" => "Tab",
        "mods.net_toggle" => "Net",
        "mods.tab_mods" => if en { "Mods" } else { "Mod" },
        "mods.tab_extensions" => if en { "Extensions" } else { "Ekstensi" },
        "mods.filter_all" => if en { "Explore" } else { "Jelajahi" },
        "mods.filter_installed" => if en { "Installed" } else { "Terpasang" },
        "mods.status_installed" => if en { "Installed" } else { "Terpasang" },
        "mods.status_available" => if en { "Available locally" } else { "Tersedia lokal" },
        "mods.status_planned" => if en { "Planned" } else { "Direncanakan" },
        "mods.toggle_active" => if en { "Active" } else { "Aktif" },
        "mods.toggle_tab" => if en { "Show tab" } else { "Tampilkan tab" },
        "mods.by" => if en { "by" } else { "oleh" },
        "mods.empty" => if en { "No community mods yet." } else { "Belum ada mod komunitas." },
        "mods.back" => if en { "‹ Back" } else { "‹ Kembali" },
        "mods.missing" => if en { "Mod is no longer available." } else { "Mod tidak tersedia lagi." },
        "mods.script_error" => if en { "Lua Mod Script Error" } else { "Kesalahan Skrip Mod Lua" },
        "notice.breathing_done" => if en { "Breathing exercise complete." } else { "Latihan napas selesai." },
        "notice.focus_scheduled" => if en { "Focus schedule started." } else { "Jadwal fokus dimulai." },
        "notice.schedule_invalid" => if en { "Schedule must be HH:MM, e.g. 09:00,14:00." } else { "Format jadwal harus HH:MM, misalnya 09:00,14:00." },
        "notice.schedule_saved" => if en { "Schedule and default tab saved." } else { "Jadwal dan tab default disimpan." },
        "notice.todo_due_invalid" => if en { "Reminder date must be YYYY-MM-DD HH:MM." } else { "Tanggal pengingat harus YYYY-MM-DD HH:MM." },
        "notice.note_saved" => if en { "Note saved." } else { "Catatan disimpan." },
        "notice.ai_saved" => if en { "AI settings saved." } else { "Pengaturan AI tersimpan." },
        "notice.ai_key_missing" => if en { "Save an API key and model first." } else { "Simpan API key dan model terlebih dahulu." },
        "notice.ai_model_missing" => if en { "Model name is empty." } else { "Nama model belum diisi." },
        "notice.calendar_feed_removed" => if en { "Calendar feed removed." } else { "Feed kalender dilepas." },
        "notice.windows_sticky_open" => if en { "Opening Windows Sticky Notes... 📌" } else { "Membuka Sticky Notes Windows... 📌" },
        "notice.windows_sticky_missing" => if en { "Windows Sticky Notes not found." } else { "Sticky Notes Windows tidak ditemukan." },
        "notice.hotkey_conflict" => if en { "Hotkey is used by another app; pick another combination." } else { "Hotkey dipakai aplikasi lain; pilih kombinasi lain." },
        "notice.media_unavailable" => if en { "Windows media is not available:" } else { "Media Windows belum tersedia:" },
        "notice.visualizer_unavailable" => if en { "Visualizer is not available:" } else { "Visualizer belum tersedia:" },
        "notice.todo_reminder" => if en { "Reminder:" } else { "Pengingat:" },
        "notice.update_check_failed" => if en { "Failed to check updates:" } else { "Gagal memeriksa update:" },
        "notice.update_available" => if en { "New version available:" } else { "Versi baru tersedia:" },
        "notice.update_latest" => if en { "mynotch is already on the latest version." } else { "mynotch sudah menggunakan versi terbaru." },
        "notice.mod_import_ok" => if en { "Installed mod:" } else { "Mod terpasang:" },
        "error.mod_install" => if en { "Failed to install mod:" } else { "Gagal memasang mod:" },
        "error.mod_uninstall" => if en { "Failed to uninstall mod:" } else { "Gagal melepas mod:" },
        "error.mod_delete" => if en { "Failed to delete mod:" } else { "Gagal menghapus mod:" },
        "error.open_folder" => if en { "Failed to open folder:" } else { "Gagal membuka folder:" },
        "error.export" => if en { "Export failed:" } else { "Gagal export:" },
        "toast.drink_confirm" => if en { "+ Drink" } else { "+ Minum" },
        "notice.at_least_one_tab" => if en { "Keep at least one active tab." } else { "Sisakan setidaknya satu tab aktif." },
        "notice.stretch_done" => if en { "Stretch recorded. See you next break." } else { "Peregangan tercatat. Sampai jeda berikutnya." },
        "notice.clip_copied" => if en { "Text copied to clipboard! 📋" } else { "Teks disalin ke clipboard! 📋" },
        "notice.clip_failed" => if en { "Failed to copy to clipboard." } else { "Gagal menyalin ke clipboard." },
        "notice.clips_cleared" => if en { "Clipboard history cleared." } else { "Riwayat clipboard dibersihkan." },
        "notice.notes_max" => if en { "Maximum 20 sticky notes reached." } else { "Maksimal 20 catatan sticky tercapai." },
        "notice.note_added" => if en { "New note added ✨" } else { "Catatan baru ditambahkan ✨" },
        "notice.note_deleted" => if en { "Note deleted." } else { "Catatan telah dihapus." },
        "notice.note_copied" => if en { "Note copied to clipboard! 📋" } else { "Isi catatan disalin ke clipboard! 📋" },
        "notice.note_popout" => if en { "Sticky note floating on desktop! 📌" } else { "Catatan sticky melayang di desktop! 📌" },
        "notice.export_md" => if en { "Exported to Markdown (.md) in data/exports/ 📄" } else { "Diexport ke Markdown (.md) di data/exports/ 📄" },
        "notice.export_txt" => if en { "Exported to TXT (.txt) in data/exports/ 📄" } else { "Diexport ke TXT (.txt) di data/exports/ 📄" },

        // Toasts & Alarms
        "toast.hydration_logged" => if en { "Great! Hydration logged" } else { "Hebat! Hidrasi tercatat" },
        "toast.break_ended" => if en { "☕ Break finished! Ready for next focus session." } else { "☕ Istirahat selesai! Siap untuk sesi fokus berikutnya." },
        "toast.focus_ended_stretch" => if en { "🎯 Focus session finished! Time to break & stretch." } else { "🎯 Sesi fokus selesai! Waktunya istirahat & peregangan." },
        "toast.focus_ended" => if en { "🎯 Focus session finished! Time for a break." } else { "🎯 Sesi fokus selesai! Waktunya istirahat." },
        "toast.stretch_alert" => if en { "🧘 Time to stretch! Stand up and move your body." } else { "🧘 Waktunya peregangan otot! Berdiri sejenak & regangkan tubuh." },
        "toast.eye_alert" => if en { "👀 20-20-20 eye break: look 20 feet away for 20 seconds." } else { "👀 Istirahat mata 20-20-20: pandang jauh 20 detik." },
        "toast.stretch_done" => if en { "Stretch finished. Ready to get back to work." } else { "Peregangan selesai. Tubuh siap kembali beraktivitas." },
        // Clipboard Search & Pin
        "clipboard.search_hint" => if en { "Search..." } else { "Cari..." },
        "clipboard.pin" => if en { "Pin" } else { "Pin" },
        "clipboard.unpin" => if en { "Unpin" } else { "Lepas Pin" },
        "notice.clip_pinned" => if en { "Clip pinned to top 📌" } else { "Klip disematkan di atas 📌" },
        "notice.clip_unpinned" => if en { "Clip unpinned." } else { "Sematkan klip dilepas." },
        "notice.clip_deleted" => if en { "Clip deleted." } else { "Klip dihapus." },
        "notice.startup_enabled" => if en { "mynotch will run at Windows startup 🚀" } else { "mynotch akan berjalan saat startup Windows 🚀" },
        "notice.startup_disabled" => if en { "mynotch removed from Windows startup." } else { "mynotch dihapus dari startup Windows." },

        // Weather
        "mod.weather.name" => if en { "Weather" } else { "Prakiraan Cuaca" },
        "mod.weather.description" => if en { "Current weather and hourly forecast (Open-Meteo)" } else { "Cuaca saat ini dan prakiraan per jam (Open-Meteo)" },
        "weather.refresh" => if en { "Refresh" } else { "Segarkan" },
        "weather.custom" => if en { "Custom" } else { "Kustom" },
        "weather.cancel" => if en { "Cancel" } else { "Batal" },
        "weather.humidity_short" => if en { "Humidity" } else { "Lembap" },
        "weather.rain_short" => if en { "Rain" } else { "Hujan" },
        "weather.wind_unit" => if en { "km/h" } else { "km/j" },
        "weather.no_hourly" => if en { "Hourly forecast unavailable." } else { "Prakiraan per jam belum tersedia." },
        "weather.no_rain_data" => if en { "Rain forecast unavailable." } else { "Prakiraan hujan belum tersedia." },
        "weather.low_rain" => if en { "Low rain chance in the next few hours." } else { "Peluang hujan rendah beberapa jam ke depan." },
        "weather.now" => if en { "Now" } else { "Kini" },
        "weather.unavailable" => if en { "Weather unavailable. Try refreshing." } else { "Cuaca belum tersedia. Coba muat ulang." },
        "weather.fetch_failed" => if en { "Could not update weather. Please try again later." } else { "Cuaca gagal diperbarui. Coba lagi nanti." },
        "weather.stale" => if en { "Last known" } else { "Data terakhir" },
        "weather.invalid_coords" => if en { "Invalid latitude/longitude." } else { "Lintang/bujur tidak valid." },
        "weather.location_hint" => if en { "Example: -6.2088, 106.8456 (Jakarta)" } else { "Contoh: -6.2088, 106.8456 (Jakarta)" },
        "weather.title" => if en { "WEATHER & FORECAST" } else { "CUACA & PRAKIRAAN" },
        "weather.updated" => if en { "Updated" } else { "Diperbarui" },
        "weather.feels_like" => if en { "Feels like" } else { "Terasa seperti" },
        "weather.humidity" => if en { "Humidity" } else { "Kelembaban" },
        "weather.wind" => if en { "Wind" } else { "Angin" },
        "weather.rain_chance" => if en { "Rain Chance" } else { "Peluang Hujan" },
        "weather.forecast_7d" => if en { "7-DAY FORECAST" } else { "PRAKIRAAN 7 HARI" },
        "weather.city" => if en { "City" } else { "Kota" },
        "weather.setup_title" => if en { "Weather Location" } else { "Pengaturan Lokasi Cuaca" },
        "weather.setup_city" => if en { "City name" } else { "Nama kota" },
        "weather.setup_lat" => if en { "Latitude" } else { "Lintang" },
        "weather.setup_lon" => if en { "Longitude" } else { "Bujur" },
        "weather.save_btn" => if en { "Save & Refresh" } else { "Simpan & Muat" },
        "weather.not_configured" => if en { "Weather location is not configured." } else { "Lokasi cuaca belum diatur." },
        "weather.loading" => if en { "Loading weather data…" } else { "Memuat data cuaca…" },
        "weather.change_city" => if en { "⚙ Change Location" } else { "⚙ Ubah Lokasi" },
        "weather.presets" => if en { "Quick Presets:" } else { "Pilihan Cepat:" },
        "stage1.weather" => if en { "Weather" } else { "Cuaca" },

        "mod.daya.name" => if en { "Power & Electricity Monitor" } else { "Monitor Daya & Listrik" },
        "mod.daya.description" => if en { "Real-time PC power consumption (Watt) and monthly PLN electricity cost accumulation" } else { "Estimasi konsumsi daya PC (Watt) dan akumulasi biaya listrik bulanan PLN" },

        "power.header" => if en { "SYSTEM POWER TELEMETRY" } else { "TELEMETRI DAYA SISTEM" },
        "power.badge" => if en { "Estimate" } else { "Estimasi" },
        "power.per_hour" => if en { "/hr (PLN)" } else { "/jam (PLN)" },
        "power.peripheral" => if en { "Peripheral" } else { "Periferal" },
        "power.monthly_title" => if en { "MONTHLY PLN ACCUMULATION" } else { "AKUMULASI BIAYA PLN BULANAN" },
        "power.this_month" => if en { "This Month" } else { "Bulan Ini" },
        "power.current_session" => if en { "Current Session" } else { "Sesi Saat Ini" },
        "power.monthly_projection" => if en { "Monthly Projection" } else { "Proyeksi Bulanan" },
        "power.active_days" => if en { "active days" } else { "hari aktif" },
        "power.tariff_info" => if en { "PLN R-1/TR Tariff Rp 1,444.70/kWh · 90% Gold PSU" } else { "Tarif PLN R-1/TR Rp 1.444,70/kWh · Efisiensi PSU 90%" },

        _ => key,
    }
}

pub fn battery_status(percentage: u8, is_charging: bool, lang: &str) -> String {
    let en = lang == "en";
    if is_charging {
        if en { format!("⚡ Charging ({percentage}%)") } else { format!("⚡ Mengisi daya ({percentage}%)") }
    } else {
        if en { format!("🔋 On battery ({percentage}%)") } else { format!("🔋 Menggunakan baterai ({percentage}%)") }
    }
}

pub fn tws_connected(name: &str, battery: Option<u8>, lang: &str) -> String {
    let state = match (lang == "en", battery) {
        (true, Some(percent)) => format!("{name} connected · {percent}%"),
        (false, Some(percent)) => format!("{name} terhubung · {percent}%"),
        (true, None) => format!("{name} connected"),
        (false, None) => format!("{name} terhubung"),
    };
    format!("🎧 {state}")
}

pub fn tws_disconnected(name: &str, lang: &str) -> String {
    if lang == "en" { format!("🎧 {name} disconnected") } else { format!("🎧 {name} terputus") }
}

pub fn weather_condition(code: u32, is_day: bool, lang: &str) -> (&'static str, &'static str) {
    let en = lang == "en";
    match code {
        0 => if is_day {
            ("☀️", if en { "Clear sky" } else { "Cerah" })
        } else {
            ("🌙", if en { "Clear night" } else { "Cerah malam" })
        },
        1 | 2 => if is_day {
            ("⛅", if en { "Partly cloudy" } else { "Sebagian berawan" })
        } else {
            ("☁️", if en { "Partly cloudy" } else { "Sebagian berawan" })
        },
        3 => ("☁️", if en { "Overcast" } else { "Berawan tebal" }),
        45 | 48 => ("🌫️", if en { "Foggy" } else { "Berkabut" }),
        51 | 53 | 55 => ("🌦️", if en { "Drizzle" } else { "Gerimis" }),
        56 | 57 => ("🌧️", if en { "Freezing drizzle" } else { "Gerimis beku" }),
        61 | 63 | 65 => ("🌧️", if en { "Rain" } else { "Hujan" }),
        66 | 67 => ("🌧️", if en { "Freezing rain" } else { "Hujan beku" }),
        71 | 73 | 75 => ("❄️", if en { "Snow" } else { "Salju" }),
        77 => ("❄️", if en { "Snow grains" } else { "Butiran salju" }),
        80 | 81 | 82 => ("🌧️", if en { "Rain showers" } else { "Hujan lebat" }),
        85 | 86 => ("❄️", if en { "Snow showers" } else { "Hujan salju" }),
        95 | 96 | 99 => ("⛈️", if en { "Thunderstorm" } else { "Badai petir" }),
        _ => ("☁️", if en { "Unavailable" } else { "Tidak tersedia" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freezing_and_snow_showers_never_claim_fair_weather_in_either_language() {
        for (code, id, en) in [(56,"Gerimis beku","Freezing drizzle"),(57,"Gerimis beku","Freezing drizzle"),(66,"Hujan beku","Freezing rain"),(67,"Hujan beku","Freezing rain"),(77,"Butiran salju","Snow grains"),(85,"Hujan salju","Snow showers"),(86,"Hujan salju","Snow showers"),(999,"Tidak tersedia","Unavailable")] {
            assert_eq!(weather_condition(code,false,"id").1,id,"WMO {code}");
            assert_eq!(weather_condition(code,false,"en").1,en,"WMO {code}");
        }
    }

    #[test]
    fn test_i18n_translations_exist_for_both_languages() {
        let keys = [
            "settings.title", "settings.auto_saved", "settings.back",
            "cat.general", "cat.appearance", "cat.tabs", "cat.audio", "cat.reminders", "cat.mods",
            "general.language", "general.fullscreen", "general.minimal", "general.default_tab", "general.startup",
            "appearance.position", "appearance.top", "appearance.bottom", "appearance.scale",
            "appearance.gap", "appearance.opacity", "appearance.material", "appearance.material.desc", "color.dark", "color.light", "material.solid", "material.lite", "material.adaptive", "material.fallback.reduced", "material.fallback.contrast", "material.fallback.software", "material.fallback.unavailable",
            "audio.visualizer", "audio.glow", "audio.romanize", "audio.open_music",
            "reminders.disabled", "reminders.enable", "reminders.configure",
            "stage1.no_music", "stage1.clips", "stage1.notes", "stage1.new_note",
            "music.no_music", "music.no_artist", "music.no_album", "music.loading_lyrics", "music.no_lyrics", "music.lyrics_error",
            "calendar.national_holiday", "calendar.collective_leave", "calendar.today_regular", "calendar.today_btn",
            "clipboard.title", "clipboard.clear", "clipboard.no_history", "clipboard.empty_title", "clipboard.plaintext_note",
            "notes.header", "notes.new_btn", "notes.no_notes", "notes.default_title",
            "reminder.remaining", "reminder.focusing", "reminder.pause", "reminder.start", "reminder.narrow",
            "notice.pinned", "notice.clip_copied", "notice.note_added",
            "mods.import_btn", "mods.folder_btn", "mods.open_btn", "mods.install_btn",
            "mods.uninstall_btn", "mods.type_builtin", "mods.by", "mods.empty", "mods.back",
            "mods.missing", "mods.script_error", "mods.tab_toggle", "mods.net_toggle",
            "mods.tab_mods", "mods.tab_extensions", "mods.filter_all", "mods.filter_installed",
            "mods.status_installed", "mods.status_available", "mods.status_planned",
            "mods.toggle_active", "mods.toggle_tab",
            "toast.hydration_logged", "toast.break_ended", "toast.stretch_alert",
        ];

        for key in keys {
            let id = t(key, "id");
            let en = t(key, "en");
            assert_ne!(id, key, "Key {key} should have Indonesian translation");
            assert_ne!(en, key, "Key {key} should have English translation");
            assert!(!id.is_empty());
            assert!(!en.is_empty());
        }
    }

    #[test]
    fn test_month_names_translation() {
        assert_eq!(month_name(5, "id"), "Mei");
        assert_eq!(month_name(5, "en"), "May");
        assert_eq!(full_month_name(5, "id"), "Mei");
        assert_eq!(full_month_name(5, "en"), "May");
        assert_eq!(month_name(8, "id"), "Ags");
        assert_eq!(month_name(8, "en"), "Aug");
        assert_eq!(full_month_name(8, "id"), "Agustus");
        assert_eq!(full_month_name(8, "en"), "August");
        assert_eq!(month_name(10, "id"), "Okt");
        assert_eq!(month_name(10, "en"), "Oct");
        assert_eq!(full_month_name(10, "id"), "Oktober");
        assert_eq!(full_month_name(10, "en"), "October");
        assert_eq!(month_name(12, "id"), "Des");
        assert_eq!(month_name(12, "en"), "Dec");
        assert_eq!(full_month_name(12, "id"), "Desember");
        assert_eq!(full_month_name(12, "en"), "December");
    }

    #[test]
    fn test_days_translation() {
        assert_eq!(short_day_name(0, "id"), "Sen");
        assert_eq!(short_day_name(0, "en"), "Mon");
        assert_eq!(day_name_upper(0, "id"), "SENIN");
        assert_eq!(day_name_upper(0, "en"), "MONDAY");
        assert_eq!(day_name_capitalized(0, "id"), "Senin");
        assert_eq!(day_name_capitalized(0, "en"), "Monday");
    }

    #[test]
    fn test_formatting_helpers_translation() {
        assert_eq!(clip_meta(100, 5, "id"), "100 kar · 5 baris");
        assert_eq!(clip_meta(100, 5, "en"), "100 chars · 5 lines");
        assert_eq!(more_lines(3, "id"), "+ 3 baris lainnya...");
        assert_eq!(more_lines(3, "en"), "+ 3 more lines...");
        assert_eq!(notes_count(7, "id"), "7 / 20 catatan");
        assert_eq!(notes_count(7, "en"), "7 / 20 notes");
        assert_eq!(focus_summary(25, "id"), "Sesi 25 menit");
        assert_eq!(focus_summary(25, "en"), "25 min session");
        assert_eq!(move_summary(45, "id"), "Setiap 45 menit");
        assert_eq!(move_summary(45, "en"), "Every 45 min");
        assert_eq!(move_desc(120, 60, "id"), "Jeda 2 menit setiap 60 menit.");
        assert_eq!(move_desc(120, 60, "en"), "2 min break every 60 min.");
        assert_eq!(battery_status(85, true, "id"), "⚡ Mengisi daya (85%)");
        assert_eq!(battery_status(85, true, "en"), "⚡ Charging (85%)");
        assert_eq!(battery_status(50, false, "id"), "🔋 Menggunakan baterai (50%)");
        assert_eq!(battery_status(50, false, "en"), "🔋 On battery (50%)");
        assert_eq!(tws_connected("Buds", Some(80), "id"), "🎧 Buds terhubung · 80%");
        assert_eq!(tws_connected("Buds", Some(80), "en"), "🎧 Buds connected · 80%");
        assert_eq!(tws_connected("Buds", None, "id"), "🎧 Buds terhubung");
        assert_eq!(tws_disconnected("Buds", "en"), "🎧 Buds disconnected");

        let (icon, cond) = weather_condition(0, true, "id");
        assert_eq!(icon, "☀️");
        assert_eq!(cond, "Cerah");

        let (icon_en, cond_en) = weather_condition(61, true, "en");
        assert_eq!(icon_en, "🌧️");
        assert_eq!(cond_en, "Rain");
    }

    fn rust_sources(dir: &std::path::Path) -> Vec<(std::path::PathBuf, String)> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&current) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if let Ok(source) = std::fs::read_to_string(&path) {
                        out.push((path, source));
                    }
                }
            }
        }
        out
    }

    /// Collects the literal keys passed to `t("...")`. Dynamic calls such as `t(key, ..)` are
    /// skipped, and a preceding `.`/identifier prevents matching unrelated `text(` methods.
    fn t_keys(source: &str) -> Vec<String> {
        let bytes = source.as_bytes();
        let mut keys = Vec::new();
        let mut i = 0;
        while i + 2 < bytes.len() {
            if bytes[i] == b't' && bytes[i + 1] == b'(' {
                let prev = i.checked_sub(1).map(|p| bytes[p]);
                let call_site = !matches!(prev, Some(c) if c.is_ascii_alphanumeric() || c == b'_' || c == b'.');
                if call_site {
                    let mut j = i + 2;
                    while j < bytes.len() && bytes[j] == b' ' { j += 1; }
                    if j < bytes.len() && bytes[j] == b'"' {
                        let start = j + 1;
                        let mut k = start;
                        while k < bytes.len() && bytes[k] != b'"' { k += 1; }
                        if k < bytes.len() {
                            if let Ok(key) = std::str::from_utf8(&bytes[start..k]) {
                                if !key.is_empty() && key.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.') {
                                    keys.push(key.to_string());
                                }
                            }
                            i = k;
                        }
                    }
                }
            }
            i += 1;
        }
        keys
    }

    #[test]
    fn every_t_key_used_in_sources_has_a_translation() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut checked = 0;
        for (path, source) in rust_sources(&root) {
            let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
            for key in t_keys(production) {
                assert_ne!(t(&key, "id"), key.as_str(), "{}: missing Indonesian translation for {key}", path.display());
                assert_ne!(t(&key, "en"), key.as_str(), "{}: missing English translation for {key}", path.display());
                checked += 1;
            }
        }
        assert!(checked > 100, "the source scan should find the t() calls, found {checked}");
    }

    #[test]
    fn sources_do_not_hardcode_the_app_version() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let stale: Vec<String> = (0..=9).map(|patch| format!("0.0.1.{patch}")).collect();
        for (path, source) in rust_sources(&root) {
            let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
            for version in &stale {
                let hits = production.lines().filter(|line| line.contains(version.as_str())).count();
                if hits == 0 { continue }
                if path.file_name().and_then(|n| n.to_str()) == Some("shell.rs") {
                    let canonical = source.lines().filter(|line| line.contains("const APP_VERSION")).count();
                    assert_eq!(hits, canonical, "{}: version {version} is hardcoded outside APP_VERSION", path.display());
                } else {
                    panic!("{} hardcodes the app version {version}; use APP_VERSION", path.display());
                }
            }
        }
    }
}

#[cfg(test)]
mod public_regression_tests {
use super::*;
#[test]
fn test_tab_names_translation() {
        assert_eq!(tab_name("Musik", "id"), "Musik");
        assert_eq!(tab_name("Musik", "en"), "Music");
        assert_eq!(tab_name("Kalender", "id"), "Kalender");
        assert_eq!(tab_name("Kalender", "en"), "Calendar");
        assert_eq!(tab_name("Reminder", "id"), "Reminder");
        assert_eq!(tab_name("Reminder", "en"), "Reminders");
        assert_eq!(tab_name("Catatan", "id"), "Catatan");
        assert_eq!(tab_name("Catatan", "en"), "Notes");
        assert_eq!(tab_name("Clipboard", "en"), "Clipboard");
        assert_eq!(tab_name("Daya", "id"), "Daya");
        assert_eq!(tab_name("Daya", "en"), "Power");
    }
}
