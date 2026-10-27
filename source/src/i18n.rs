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
        "appearance.frosted" => if en { "Frosted Glass effect" } else { "Efek Frosted Glass" },
        "appearance.frosted.desc" => if en { "Glass sheen and crystal inner rim when translucent." } else { "Kilau kaca dan garis tepi kristal saat transparan." },

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
            "appearance.gap", "appearance.opacity", "appearance.frosted",
            "audio.visualizer", "audio.glow", "audio.romanize", "audio.open_music",
            "reminders.disabled", "reminders.enable", "reminders.configure",
            "stage1.no_music", "stage1.clips", "stage1.notes", "stage1.new_note",
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
        assert_eq!(month_name(8, "id"), "Ags");
        assert_eq!(month_name(8, "en"), "Aug");
        assert_eq!(month_name(10, "id"), "Okt");
        assert_eq!(month_name(10, "en"), "Oct");
        assert_eq!(month_name(12, "id"), "Des");
        assert_eq!(month_name(12, "en"), "Dec");
    }
}
