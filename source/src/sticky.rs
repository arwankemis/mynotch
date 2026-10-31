use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const NOTE_COLORS: [(u32, &'static str); 5] = [
    (0xfef08a, "Kuning"), // Pastel yellow
    (0xbae6fd, "Biru"),   // Pastel sky blue
    (0xbbf7d0, "Hijau"),  // Pastel green
    (0xfbcfe8, "Pink"),   // Pastel pink
    (0xe9d5ff, "Ungu"),   // Pastel purple
];

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct StickyNote {
    pub id: u64,
    pub title: String,
    pub content: String,
    pub color_idx: u8,
    pub updated_at: String,
}

impl Default for StickyNote {
    fn default() -> Self {
        Self {
            id: 0,
            title: "Catatan Baru".into(),
            content: String::new(),
            color_idx: 0,
            updated_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let mut cleaned = String::with_capacity(name.len());
    let mut last_space = false;
    for c in name.chars() {
        if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
            if !last_space && !cleaned.is_empty() {
                cleaned.push(' ');
                last_space = true;
            }
        } else if c.is_whitespace() {
            if !last_space && !cleaned.is_empty() {
                cleaned.push(' ');
                last_space = true;
            }
        } else {
            cleaned.push(c);
            last_space = false;
        }
    }
    let res = cleaned.trim().to_string();
    if res.is_empty() {
        "catatan".to_string()
    } else {
        res
    }
}

/// Writes `contents` to `path` atomically: a temp file in the same directory is renamed
/// over the destination, so a crash mid-export never leaves a half-written note file
/// behind. The previous export is replaced (no .bak is kept).
fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    let mut tmp_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("export").to_string();
    tmp_name.push_str(".tmp");
    let tmp = path.with_file_name(tmp_name);
    if let Err(e) = std::fs::write(&tmp, contents) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    Ok(())
}

pub fn export_note_to_markdown(exports_dir: &Path, note: &StickyNote) -> Result<PathBuf, String> {
    std::fs::create_dir_all(exports_dir).map_err(|e| e.to_string())?;
    let safe_title = sanitize_filename(&note.title);
    let filename = format!("{}-{}.md", safe_title.replace(' ', "-"), note.id);
    let path = exports_dir.join(filename);

    let color_name = NOTE_COLORS.get(note.color_idx as usize).map(|c| c.1).unwrap_or("Kuning");
    let md = format!(
        "# {}\n\n> Diperbarui: {} · Warna: {}\n\n{}\n",
        if note.title.trim().is_empty() { "Tanpa Judul" } else { &note.title },
        note.updated_at,
        color_name,
        note.content
    );

    atomic_write(&path, &md)?;
    Ok(path)
}

pub fn export_note_to_txt(exports_dir: &Path, note: &StickyNote) -> Result<PathBuf, String> {
    std::fs::create_dir_all(exports_dir).map_err(|e| e.to_string())?;
    let safe_title = sanitize_filename(&note.title);
    let filename = format!("{}-{}.txt", safe_title.replace(' ', "-"), note.id);
    let path = exports_dir.join(filename);

    let title_line = if note.title.trim().is_empty() { "Tanpa Judul" } else { &note.title };
    let sep = "=".repeat(title_line.chars().count().max(20));
    let txt = format!(
        "{}\n{}\nDiperbarui: {}\n\n{}\n",
        title_line,
        sep,
        note.updated_at,
        note.content
    );

    atomic_write(&path, &txt)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_removes_illegal_characters() {
        assert_eq!(sanitize_filename("Ide: Project/2026? *super* <cool> | \"wow\""), "Ide Project 2026 super cool wow");
        assert_eq!(sanitize_filename("   "), "catatan");
    }

    #[test]
    fn test_export_note_to_markdown_creates_file_with_metadata() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_export_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let note = StickyNote {
            id: 12345,
            title: "Ide Project".into(),
            content: "Baris 1\nBaris 2".into(),
            color_idx: 0,
            updated_at: "2026-10-27 21:00".into(),
        };

        let file_path = export_note_to_markdown(&temp_dir, &note).unwrap();
        assert!(file_path.exists());
        assert!(file_path.extension().and_then(|s| s.to_str()) == Some("md"));
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("# Ide Project"));
        assert!(content.contains("Baris 1\nBaris 2"));
        assert!(content.contains("2026-10-27 21:00"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_note_to_txt_creates_plain_file() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_export_txt_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let note = StickyNote {
            id: 67890,
            title: "Daftar Belanja".into(),
            content: "- Kopi\n- Susu".into(),
            color_idx: 1,
            updated_at: "2026-10-27 21:05".into(),
        };

        let file_path = export_note_to_txt(&temp_dir, &note).unwrap();
        assert!(file_path.exists());
        assert!(file_path.extension().and_then(|s| s.to_str()) == Some("txt"));
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Daftar Belanja"));
        assert!(content.contains("- Kopi\n- Susu"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_sticky_note_default_and_color_palette() {
        let note = StickyNote::default();
        assert_eq!(note.title, "Catatan Baru");
        assert_eq!(note.color_idx, 0);
        assert_eq!(NOTE_COLORS.len(), 5);
        assert_eq!(NOTE_COLORS[0].1, "Kuning");
        assert_eq!(NOTE_COLORS[1].1, "Biru");
        assert_eq!(NOTE_COLORS[2].1, "Hijau");
        assert_eq!(NOTE_COLORS[3].1, "Pink");
        assert_eq!(NOTE_COLORS[4].1, "Ungu");
    }

    #[test]
    fn test_sticky_notes_delete_and_add_state_lifecycle() {
        let mut notes: Vec<StickyNote> = Vec::new();
        notes.insert(0, StickyNote {
            id: 1,
            title: "Catatan Lama".into(),
            content: "Teks yang harusnya dihapus".into(),
            color_idx: 0,
            updated_at: "2026-10-27 21:00".into(),
        });
        assert_eq!(notes.len(), 1);

        notes.retain(|n| n.id != 1);
        assert_eq!(notes.len(), 0);

        let new_note = StickyNote::default();
        notes.insert(0, new_note);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Catatan Baru");
        assert_eq!(notes[0].content, "");
        assert_ne!(notes[0].title, "Catatan Lama");
    }
}
