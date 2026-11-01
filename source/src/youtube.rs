//! YouTube caption provider: fetches and converts YouTube timedtext captions to synchronized LRC lyrics.
use crate::net;
use crate::online::LyricsResult;
use serde_json::Value;

pub fn fetch_captions(title: &str, artist: &str, duration: f64) -> Result<LyricsResult, String> {
    net::check_cancel()?;
    if title.trim().is_empty() {
        return Err("Judul video kosong.".into());
    }

    // 1. Search YouTube for video ID
    let query = if artist.trim().is_empty() {
        title.trim().to_string()
    } else {
        format!("{} {}", title.trim(), artist.trim())
    };

    let encoded_query = url_encode(&query);
    let search_url = format!("https://www.youtube.com/results?search_query={encoded_query}");
    let search_html = match net::request(
        "GET",
        &search_url,
        &[
            ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"),
            ("Accept-Language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7"),
        ],
        &[],
    ) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(e) => return Err(format!("Pencarian YouTube gagal: {e}")),
    };

    let vids = extract_video_ids(&search_html);
    if vids.is_empty() {
        return Err("Video YouTube tidak ditemukan.".into());
    }

    // 2. Try candidate video IDs
    for vid in vids.iter().take(3) {
        net::check_cancel()?;
        if let Ok(result) = fetch_video_captions(vid, title, artist, duration) {
            if !result.synced_lyrics.is_empty() || !result.plain_lyrics.is_empty() {
                return Ok(result);
            }
        }
    }

    Err("Caption YouTube tidak tersedia untuk video ini.".into())
}

/// Checks whether any visible top-level window contains "YouTube" in its title.
/// Used to detect YouTube media sessions playing inside standard web browsers.
pub fn is_youtube_window_present() -> bool {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, IsWindowVisible};

    struct Ctx {
        found: bool,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let ctx = &mut *(lparam.0 as *mut Ctx);
            if IsWindowVisible(hwnd).as_bool() {
                let mut buf = [0u16; 256];
                let len = GetWindowTextW(hwnd, &mut buf);
                if len > 0 {
                    let title = String::from_utf16_lossy(&buf[..len as usize]);
                    if title.contains("YouTube") {
                        ctx.found = true;
                        return BOOL(0);
                    }
                }
            }
            BOOL(1)
        }
    }

    let mut ctx = Ctx { found: false };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut _ as isize));
    }
    ctx.found
}

fn fetch_video_captions(video_id: &str, title: &str, artist: &str, duration: f64) -> Result<LyricsResult, String> {
    let payload = serde_json::json!({
        "context": {
            "client": {
                "clientName": "ANDROID",
                "clientVersion": "20.10.38",
                "hl": "id",
                "gl": "ID"
            }
        },
        "videoId": video_id
    });

    let payload_bytes = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let resp_bytes = net::request(
        "POST",
        "https://www.youtube.com/youtubei/v1/player?prettyPrint=false",
        &[
            ("Content-Type", "application/json"),
            ("User-Agent", "com.google.android.youtube/20.10.38 (Linux; U; Android 14) gzip"),
        ],
        &payload_bytes,
    )?;

    let json: Value = serde_json::from_slice(&resp_bytes).map_err(|e| format!("JSON player tidak valid: {e}"))?;
    let caption_tracks = json
        .get("captions")
        .and_then(|c| c.get("playerCaptionsTracklistRenderer"))
        .and_then(|r| r.get("captionTracks"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| "Tidak ada caption track".to_string())?;

    if caption_tracks.is_empty() {
        return Err("Daftar caption kosong.".into());
    }

    // Pick best track: Indonesian first, then English, then first available
    let mut selected_url = None;
    for lang in ["id", "en"] {
        for track in caption_tracks {
            let code = track.get("languageCode").and_then(|c| c.as_str()).unwrap_or("");
            if code.starts_with(lang) {
                if let Some(url) = track.get("baseUrl").and_then(|u| u.as_str()) {
                    selected_url = Some(url.to_string());
                    break;
                }
            }
        }
        if selected_url.is_some() {
            break;
        }
    }

    if selected_url.is_none() {
        if let Some(first) = caption_tracks.first() {
            selected_url = first.get("baseUrl").and_then(|u| u.as_str()).map(|s| s.to_string());
        }
    }

    let caption_url = selected_url.ok_or_else(|| "URL caption tidak valid.".to_string())?;
    let xml_bytes = net::request(
        "GET",
        &caption_url,
        &[("User-Agent", "com.google.android.youtube/20.10.38 (Linux; U; Android 14) gzip")],
        &[],
    )?;

    let xml_text = String::from_utf8_lossy(&xml_bytes);
    parse_timedtext(&xml_text, 40, title, artist, duration)
}

pub fn extract_video_ids(html: &str) -> Vec<String> {
    let mut vids = Vec::new();
    let marker = "\"videoId\":\"";
    let mut cursor = 0;
    while let Some(pos) = html[cursor..].find(marker) {
        let start = cursor + pos + marker.len();
        if start + 11 <= html.len() {
            let id = &html[start..start + 11];
            if id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
                if !vids.iter().any(|v: &String| v == id) {
                    vids.push(id.to_string());
                }
            }
        }
        cursor = start + 11;
        if vids.len() >= 10 {
            break;
        }
    }
    vids
}

pub fn parse_timedtext(xml: &str, max_line_len: usize, _title: &str, _artist: &str, _duration: f64) -> Result<LyricsResult, String> {
    let max_len = if max_line_len == 0 { 40 } else { max_line_len };
    let mut synced_lines = Vec::new();
    let mut plain_paragraphs = Vec::new();

    // Iterate through <p ...> tags
    let mut p_cursor = 0;
    while let Some(p_start_rel) = xml[p_cursor..].find("<p ") {
        let p_start = p_cursor + p_start_rel;
        let p_close = match xml[p_start..].find('>') {
            Some(idx) => p_start + idx,
            None => break,
        };
        let p_end = match xml[p_close..].find("</p>") {
            Some(idx) => p_close + idx,
            None => break,
        };

        let tag_header = &xml[p_start..p_close];
        let p_body = &xml[p_close + 1..p_end];
        p_cursor = p_end + 4;

        let p_t = extract_attribute_u64(tag_header, "t").unwrap_or(0) as f64 / 1000.0;
        let p_d = extract_attribute_u64(tag_header, "d").unwrap_or(2000) as f64 / 1000.0;

        // Check if there are word tags <s ...> inside <p>
        let mut words_with_time = Vec::new();
        if p_body.contains("<s") {
            let mut s_cursor = 0;
            while let Some(s_start_rel) = p_body[s_cursor..].find("<s") {
                let s_start = s_cursor + s_start_rel;
                let s_tag_close = match p_body[s_start..].find('>') {
                    Some(idx) => s_start + idx,
                    None => break,
                };
                let s_end = match p_body[s_tag_close..].find("</s>") {
                    Some(idx) => s_tag_close + idx,
                    None => break,
                };

                let s_header = &p_body[s_start..s_tag_close];
                let s_text = unescape_xml(&p_body[s_tag_close + 1..s_end]);
                s_cursor = s_end + 4;

                let offset_ms = extract_attribute_u64(s_header, "t").unwrap_or(0) as f64 / 1000.0;
                let word_time = p_t + offset_ms;
                let trimmed = s_text.trim();
                if !trimmed.is_empty() {
                    words_with_time.push((word_time, trimmed.to_string()));
                }
            }
        }

        if !words_with_time.is_empty() {
            // Build lines respecting max_len limit
            let mut chunk: Vec<String> = Vec::new();
            let mut chunk_start = words_with_time[0].0;
            let mut current_len = 0;

            for (w_time, word) in words_with_time {
                let additional = if chunk.is_empty() { word.len() } else { 1 + word.len() };
                if current_len + additional > max_len && !chunk.is_empty() {
                    // Flush line
                    let text = chunk.join(" ");
                    let lrc = format!("[{}]{}", format_lrc_timestamp(chunk_start), text);
                    synced_lines.push(lrc);
                    plain_paragraphs.push(text);

                    chunk.clear();
                    chunk_start = w_time;
                    current_len = 0;
                }

                current_len += if chunk.is_empty() { word.len() } else { 1 + word.len() };
                chunk.push(word);
            }

            if !chunk.is_empty() {
                let text = chunk.join(" ");
                let lrc = format!("[{}]{}", format_lrc_timestamp(chunk_start), text);
                synced_lines.push(lrc);
                plain_paragraphs.push(text);
            }
        } else {
            // Plain text without <s> tags
            let clean_text = unescape_xml(&strip_xml_tags(p_body));
            let trimmed = clean_text.trim();
            if !trimmed.is_empty() {
                if trimmed.chars().count() <= max_len {
                    let lrc = format!("[{}]{}", format_lrc_timestamp(p_t), trimmed);
                    synced_lines.push(lrc);
                    plain_paragraphs.push(trimmed.to_string());
                } else {
                    // Split sentence into words and interpolate timestamps
                    let words: Vec<&str> = trimmed.split_whitespace().collect();
                    let mut chunk: Vec<&str> = Vec::new();
                    let mut chunk_len = 0;
                    let mut sub_chunks: Vec<String> = Vec::new();

                    for w in words {
                        let add = if chunk.is_empty() { w.len() } else { 1 + w.len() };
                        if chunk_len + add > max_len && !chunk.is_empty() {
                            sub_chunks.push(chunk.join(" "));
                            chunk.clear();
                            chunk_len = 0;
                        }
                        chunk_len += if chunk.is_empty() { w.len() } else { 1 + w.len() };
                        chunk.push(w);
                    }
                    if !chunk.is_empty() {
                        sub_chunks.push(chunk.join(" "));
                    }

                    let n_chunks = sub_chunks.len().max(1);
                    for (i, sub) in sub_chunks.into_iter().enumerate() {
                        let chunk_t = p_t + (p_d * i as f64 / n_chunks as f64);
                        let lrc = format!("[{}]{}", format_lrc_timestamp(chunk_t), sub);
                        synced_lines.push(lrc);
                        plain_paragraphs.push(sub);
                    }
                }
            }
        }
    }

    if synced_lines.is_empty() {
        return Err("Tidak ada baris caption yang dapat diekstrak.".into());
    }

    Ok(LyricsResult {
        plain_lyrics: plain_paragraphs.join("\n"),
        synced_lyrics: synced_lines.join("\n"),
        instrumental: false,
        is_youtube: true,
    })
}

fn extract_attribute_u64(tag: &str, attr: &str) -> Option<u64> {
    let pattern = format!("{attr}=\"");
    let start = tag.find(&pattern)? + pattern.len();
    let end = tag[start..].find('"')? + start;
    tag[start..end].parse().ok()
}

pub fn format_lrc_timestamp(seconds: f64) -> String {
    let total_secs = seconds.max(0.0);
    let mins = (total_secs / 60.0).floor() as u64;
    let rem_secs = total_secs - (mins as f64 * 60.0);
    let secs = rem_secs.floor() as u64;
    let centis = ((rem_secs - secs as f64) * 100.0).floor() as u64;
    format!("{mins:02}:{secs:02}.{centis:02}")
}

pub fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#10;", " ")
        .replace("&#13;", "")
}

pub fn strip_xml_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut inside = false;
    for c in s.chars() {
        if c == '<' {
            inside = true;
        } else if c == '>' {
            inside = false;
        } else if !inside {
            out.push(c);
        }
    }
    out
}

fn url_encode(input: &str) -> String {
    let mut result = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            result.push(byte as char);
        } else if byte == b' ' {
            result.push('+');
        } else {
            result.push_str(&format!("%{byte:02X}"));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_ids() {
        let sample = r#"something "videoId":"wAvlqGbaCw0" and another "videoId":"VcEX_90Sc8I" repeated "videoId":"wAvlqGbaCw0""#;
        let vids = extract_video_ids(sample);
        assert_eq!(vids, vec!["wAvlqGbaCw0", "VcEX_90Sc8I"]);
    }

    #[test]
    fn test_format_lrc_timestamp() {
        assert_eq!(format_lrc_timestamp(0.0), "00:00.00");
        assert_eq!(format_lrc_timestamp(65.43), "01:05.43");
        assert_eq!(format_lrc_timestamp(125.0), "02:05.00");
    }

    #[test]
    fn test_parse_timedtext_with_word_splits() {
        let xml = r#"<?xml version="1.0" encoding="utf-8" ?>
<timedtext format="3">
<body>
<p t="1000" d="6000">
  <s ac="0">Jurusan</s>
  <s t="500" ac="0"> paling</s>
  <s t="1000" ac="0"> dibutuhin</s>
  <s t="1500" ac="0"> di</s>
  <s t="2000" ac="0"> era</s>
  <s t="2500" ac="0"> kecerdasan</s>
  <s t="3000" ac="0"> buatan</s>
  <s t="3500" ac="0"> ternyata</s>
  <s t="4000" ac="0"> bukan</s>
  <s t="4500" ac="0"> teknik</s>
  <s t="5000" ac="0"> informatika</s>
</p>
</body>
</timedtext>"#;

        let res = parse_timedtext(xml, 35, "Title", "Artist", 10.0).unwrap();
        let lines: Vec<&str> = res.synced_lyrics.lines().collect();
        assert!(lines.len() >= 2, "Long sentence must split into multiple lines, got: {:?}", lines);
        for line in &lines {
            // Strip [mm:ss.xx] timestamp
            let text = &line[10..];
            assert!(text.chars().count() <= 35, "Line too long ({} chars): {}", text.chars().count(), text);
        }
        assert_eq!(lines[0], "[00:01.00]Jurusan paling dibutuhin di era");
        assert_eq!(lines[1], "[00:03.50]kecerdasan buatan ternyata bukan");
    }

    #[test]
    fn test_parse_timedtext_plain_fallback_splits() {
        let xml = r#"<timedtext format="3"><body>
<p t="2000" d="4000">Kalimat ini sengaja dibuat sangat panjang sekali agar terpicu pemotongan otomatis menjadi beberapa baris yang rapi.</p>
</body></timedtext>"#;

        let res = parse_timedtext(xml, 35, "Title", "Artist", 10.0).unwrap();
        let lines: Vec<&str> = res.synced_lyrics.lines().collect();
        assert!(lines.len() >= 3, "Expected at least 3 chunks, got: {:?}", lines);
        for line in &lines {
            let text = &line[10..];
            assert!(text.chars().count() <= 35, "Line text too long ({} chars): {}", text.chars().count(), text);
        }
    }
}
