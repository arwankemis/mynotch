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
        "cat.general.desc" => if en { "mynotch 0.0.1 alpha · Portable Windows." } else { "mynotch 0.0.1 alpha · Portable Windows." },
        "cat.appearance" => if en { "Appearance" } else { "Tampilan" },
        "cat.appearance.desc" => if en { "Position and visual comfort." } else { "Posisi dan kenyamanan tampilan." },
        "cat.tabs" => if en { "Tabs & navigation" } else { "Tab & navigasi" },
        "cat.tabs.desc" => if en { "Reorder tabs; keep at least one active." } else { "Atur urutan; sisakan satu tab aktif." },
        "cat.audio" => if en { "Music" } else { "Musik" },
        "cat.audio.desc" => if en { "Player, lyrics, and audio visualization." } else { "Pemutar, lirik, dan visualisasi audio." },
        "cat.reminders" => if en { "Reminders" } else { "Reminder" },
        "cat.reminders.desc" => if en { "Daily reminder and health settings." } else { "Buka pengaturan pengingat harian." },
        "cat.mods" => if en { "Mods & Extensions" } else { "Mod & Ekstensi" },
        "cat.mods.desc" => if en { "Install or uninstall optional modular features." } else { "Pasang atau lepas fitur modular opsional." },

        // Settings General
        "general.language" => if en { "Language" } else { "Bahasa" },
        "general.language.desc" => if en { "Select application interface language." } else { "Pilih bahasa antarmuka aplikasi." },
        "general.fullscreen" => if en { "Hide in fullscreen" } else { "Sembunyikan saat layar penuh" },
        "general.fullscreen.desc" => if en { "When gaming or watching videos." } else { "Saat bermain game atau menonton video." },
        "general.minimal" => if en { "Minimal bar mode" } else { "Mode bar kecil" },
        "general.minimal.desc" => if en { "Use a sleek mini bar when notch is collapsed." } else { "Gunakan bar kecil saat notch ditutup." },
        "general.default_tab" => if en { "Startup tab" } else { "Tab saat dibuka" },
        "general.default_tab.desc" => if en { "Initial tab when application starts." } else { "Halaman awal ketika aplikasi berjalan." },

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
        "notice.mod_installed" => if en { "Mod installed! New tab available." } else { "Mod berhasil dipasang! Tab baru tersedia." },
        "notice.mod_uninstalled" => if en { "Mod uninstalled." } else { "Mod berhasil dilepas." },
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

        _ => key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_translations_exist_for_both_languages() {
        let keys = [
            "settings.title", "settings.auto_saved", "settings.back",
            "cat.general", "cat.appearance", "cat.tabs", "cat.audio", "cat.reminders", "cat.mods",
            "general.language", "general.fullscreen", "general.minimal", "general.default_tab",
            "appearance.position", "appearance.top", "appearance.bottom", "appearance.scale",
            "appearance.gap", "appearance.opacity",
            "audio.visualizer", "audio.glow", "audio.romanize", "audio.open_music",
            "reminders.disabled", "reminders.enable", "reminders.configure",
            "stage1.no_music", "stage1.clips", "stage1.notes", "stage1.new_note",
            "music.no_music", "music.no_artist", "music.no_album", "music.loading_lyrics", "music.no_lyrics",
            "calendar.national_holiday", "calendar.collective_leave", "calendar.today_regular", "calendar.today_btn",
            "clipboard.title", "clipboard.clear", "clipboard.no_history", "clipboard.empty_title",
            "notes.header", "notes.new_btn", "notes.no_notes", "notes.default_title",
            "reminder.remaining", "reminder.focusing", "reminder.pause", "reminder.start",
            "notice.pinned", "notice.clip_copied", "notice.note_added",
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
    }
}
