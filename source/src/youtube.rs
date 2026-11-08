//! YouTube caption provider: fetches and converts YouTube timedtext captions to synchronized LRC lyrics.
use crate::net;
use crate::online::LyricsResult;
use serde_json::Value;

pub fn fetch_captions(title: &str, artist: &str, duration: f64, preferred_lang: &str, permission: &crate::lyrics_gate::LyricsPermission) -> Result<LyricsResult, String> {
    permission.check()?;
    if !permission.youtube() { return Err("YouTube consent disabled".into()); }
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
    permission.check()?;
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

    // Never accept the first search hit solely because it has captions. Multiple
    // matching videos are ambiguous without an ID supplied by the media session.
    let mut matched = None;
    for vid in vids.iter().take(3) {
        net::check_cancel()?;
        if let Ok(result) = fetch_video_captions(vid, title, artist, duration, preferred_lang, permission) {
            if !result.synced_lyrics.is_empty() || !result.plain_lyrics.is_empty() {
                if matched.is_some() { return Err("Beberapa video cocok; caption tidak dapat dipastikan untuk sesi ini.".into()); }
                matched = Some(result);
            }
        }
    }

    matched.ok_or_else(|| "Caption YouTube tidak tersedia atau identitas video tidak dapat diverifikasi.".into())
}

/// Only metadata belonging to this session may enable caption lookup.
pub fn session_allows_captions(source: &str, title: &str, album: &str) -> bool {
    let source_low = source.to_lowercase();
    if source_low.contains("youtube") {
        return true;
    }
    let is_browser = ["chrome", "edge", "firefox", "opera", "brave", "vivaldi", "browser"]
        .iter()
        .any(|browser| source_low.contains(browser));
    if !is_browser {
        return false;
    }
    if [title, album].iter().any(|value| value.to_lowercase().contains("youtube")) {
        return true;
    }
    #[cfg(target_os = "windows")]
    {
        if browser_has_window_keyword("youtube", title) {
            return true;
        }
    }
    false
}

/// Detects if the media session is from Twitch (standalone app or web stream).
pub fn is_twitch_session(source: &str, title: &str, album: &str) -> bool {
    let source_low = source.to_lowercase();
    if source_low.contains("twitch") {
        return true;
    }
    let is_browser = ["chrome", "edge", "firefox", "opera", "brave", "vivaldi", "browser"]
        .iter()
        .any(|browser| source_low.contains(browser));
    if !is_browser {
        return false;
    }
    if [title, album].iter().any(|value| value.to_lowercase().contains("twitch")) {
        return true;
    }
    #[cfg(target_os = "windows")]
    {
        if browser_has_window_keyword("twitch", title) {
            return true;
        }
    }
    false
}

#[cfg(target_os = "windows")]
pub fn browser_has_window_keyword(keyword: &str, track_title: &str) -> bool {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextLengthW, GetWindowTextW};

    struct Context<'a> {
        keyword: &'a str,
        title_prefix: Option<String>,
        keyword_matched: bool,
        exact_matched: bool,
    }

    let prefix = {
        let trimmed = track_title.trim();
        if trimmed.len() >= 4 {
            let end = trimmed.char_indices().nth(12).map(|(i, _)| i).unwrap_or(trimmed.len());
            Some(trimmed[..end].to_lowercase())
        } else {
            None
        }
    };

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 {
                return BOOL(1);
            }
            let ctx = &mut *(lparam.0 as *mut Context);
            let mut buf = vec![0u16; (len + 1) as usize];
            let copied = GetWindowTextW(hwnd, &mut buf);
            if copied > 0 {
                let text = String::from_utf16_lossy(&buf[..copied as usize]);
                let text_low = text.to_lowercase();
                if text_low.contains(ctx.keyword) {
                    ctx.keyword_matched = true;
                    if let Some(ref pfx) = ctx.title_prefix {
                        if text_low.contains(pfx) {
                            ctx.exact_matched = true;
                            return BOOL(0);
                        }
                    }
                }
            }
            BOOL(1)
        }
    }

    let mut ctx = Context {
        keyword,
        title_prefix: prefix,
        keyword_matched: false,
        exact_matched: false,
    };
    unsafe {
        let _ = EnumWindows(Some(enum_cb), LPARAM(&mut ctx as *mut _ as isize));
    }
    ctx.exact_matched || ctx.keyword_matched
}

fn clean_video_title(title: &str) -> String {
    let mut title = title.to_lowercase();
    for decoration in ["(official music video)", "[official music video]", "(official video)", "[official video]", "(official audio)", "[official audio]", "(lyrics)", "[lyrics]"] {
        title = title.replace(decoration, " ");
    }
    crate::online::normalize_track_text(&title)
}

pub(crate) fn is_allowed_caption_host(url: &str) -> bool {
    if !url.starts_with("https://") {
        return false;
    }
    let rest = match url.strip_prefix("https://") {
        Some(r) => r,
        None => return false,
    };
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let host = match authority.rsplit_once(':') {
        Some((h, _)) => h,
        None => authority,
    };
    let host_low = host.to_ascii_lowercase();
    host_low == "youtube.com"
        || host_low.ends_with(".youtube.com")
        || host_low == "googlevideo.com"
        || host_low.ends_with(".googlevideo.com")
}

pub(crate) fn verify_candidate_match(details: &Value, title: &str, artist: &str, duration: f64) -> Result<(), String> {
    let length = details["lengthSeconds"].as_str().and_then(|s| s.parse::<f64>().ok())
        .filter(|n| n.is_finite() && *n > 0.0).ok_or("Durasi video tidak tersedia.")?;
    if !duration.is_finite() || duration <= 0.0 || (length - duration).abs() > 2.0 {
        return Err("Durasi video tidak cocok dengan sesi media.".into());
    }
    let wanted = clean_video_title(title);
    let actual = clean_video_title(details["title"].as_str().ok_or("Judul video tidak tersedia.")?);
    let artist = crate::online::normalize_track_text(artist);
    if wanted.is_empty() || artist.is_empty() { return Err("Identitas media belum cukup untuk memverifikasi caption.".into()); }
    let author = crate::online::normalize_track_text(details["author"].as_str().unwrap_or("").trim_end_matches(" - Topic"));
    let title_matches = actual == wanted || actual == format!("{artist} {wanted}");
    let artist_matches = author == artist || actual.starts_with(&format!("{artist} "));
    if !title_matches || !artist_matches { return Err("Judul atau artis video tidak cocok dengan sesi media.".into()); }
    Ok(())
}

fn fetch_video_captions(video_id: &str, title: &str, artist: &str, duration: f64, preferred_lang: &str, permission: &crate::lyrics_gate::LyricsPermission) -> Result<LyricsResult, String> {
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
    permission.check()?;
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

    let details = json.get("videoDetails").ok_or("Identitas video tidak tersedia.")?;
    if details["videoId"].as_str() != Some(video_id) { return Err("ID respons video tidak cocok.".into()); }
    verify_candidate_match(details, title, artist, duration)?;

    let caption_tracks = json
        .get("captions")
        .and_then(|c| c.get("playerCaptionsTracklistRenderer"))
        .and_then(|r| r.get("captionTracks"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| "Tidak ada caption track".to_string())?;

    if caption_tracks.is_empty() {
        return Err("Daftar caption kosong.".into());
    }

    let mut selected_url = None;
    match preferred_lang {
        "id" => {
            for track in caption_tracks {
                let code = track.get("languageCode").and_then(|c| c.as_str()).unwrap_or("");
                if code.starts_with("id") {
                    selected_url = track.get("baseUrl").and_then(|u| u.as_str()).map(str::to_string);
                    break;
                }
            }
            if selected_url.is_none() {
                for track in caption_tracks {
                    let translatable = track.get("isTranslatable").and_then(|v| v.as_bool()).unwrap_or(false);
                    if translatable {
                        if let Some(base) = track.get("baseUrl").and_then(|u| u.as_str()) {
                            selected_url = Some(format!("{base}&tlang=id"));
                            break;
                        }
                    }
                }
            }
        },
        "en" => {
            for track in caption_tracks {
                let code = track.get("languageCode").and_then(|c| c.as_str()).unwrap_or("");
                if code.starts_with("en") {
                    selected_url = track.get("baseUrl").and_then(|u| u.as_str()).map(str::to_string);
                    break;
                }
            }
            if selected_url.is_none() {
                for track in caption_tracks {
                    let translatable = track.get("isTranslatable").and_then(|v| v.as_bool()).unwrap_or(false);
                    if translatable {
                        if let Some(base) = track.get("baseUrl").and_then(|u| u.as_str()) {
                            selected_url = Some(format!("{base}&tlang=en"));
                            break;
                        }
                    }
                }
            }
        },
        _ => {
            // "orig" / default: prefer manual/non-ASR track, or first track
            for track in caption_tracks {
                let kind = track.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                if kind != "asr" {
                    selected_url = track.get("baseUrl").and_then(|u| u.as_str()).map(str::to_string);
                    break;
                }
            }
            if selected_url.is_none() {
                if let Some(first) = caption_tracks.first() {
                    selected_url = first.get("baseUrl").and_then(|u| u.as_str()).map(str::to_string);
                }
            }
        }
    }

    if selected_url.is_none() {
        if let Some(first) = caption_tracks.first() {
            selected_url = first.get("baseUrl").and_then(|u| u.as_str()).map(|s| s.to_string());
        }
    }

    let caption_url = selected_url.ok_or_else(|| "URL caption tidak valid.".to_string())?;
    if !is_allowed_caption_host(&caption_url) {
        return Err(format!("Host URL caption tidak diizinkan: {caption_url}"));
    }
    // Request the paragraph format that the parser understands explicitly.
    let caption_url = caption_srv3_url(&caption_url);
    permission.check()?;
    let xml_bytes = net::request(
        "GET",
        &caption_url,
        &[("User-Agent", "com.google.android.youtube/20.10.38 (Linux; U; Android 14) gzip")],
        &[],
    )?;

    let xml_text = String::from_utf8_lossy(&xml_bytes);
    parse_timedtext(&xml_text, 40, title, artist, duration)
}

fn caption_srv3_url(url: &str) -> String {
    let (base, query) = url.split_once('?').unwrap_or((url, ""));
    let mut params: Vec<&str> = query.split('&').filter(|p| !p.is_empty() && !p.starts_with("fmt=")).collect();
    params.push("fmt=srv3");
    format!("{base}?{}", params.join("&"))
}

pub fn extract_video_ids(html: &str) -> Vec<String> {
    let mut vids = Vec::new();
    for rest in html.split("\"videoId\":\"").skip(1) {
        let Some(id) = rest.get(..11) else { continue; };
        if rest.as_bytes().get(11) == Some(&b'"') && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') && !vids.iter().any(|v| v == id) {
            vids.push(id.to_string());
        }
        if vids.len() >= 10 { break; }
    }
    vids
}

pub fn parse_timedtext(xml: &str, _max_line_len: usize, _title: &str, _artist: &str, _duration: f64) -> Result<LyricsResult, String> {
    if xml.len() > 1024 * 1024 { return Err("Caption melebihi batas ukuran.".into()); }
    let mut synced_lines = Vec::new();
    let mut plain_lines = Vec::new();
    let mut cursor = 0;
    while let Some(start) = xml[cursor..].find("<p ") {
        let start = cursor + start;
        let Some(close) = xml[start..].find('>').map(|i| start + i) else { break; };
        let Some(end) = xml[close..].find("</p>").map(|i| close + i) else { break; };
        cursor = end + 4;
        let Some(at) = extract_attribute_u64(&xml[start..close], "t") else { continue; };
        let body = &xml[close + 1..end];
        // Keep paragraph timing intact. Word spans are presentation, not new lyric lines.
        let text = unescape_xml(&strip_xml_tags(body)).split_whitespace().collect::<Vec<_>>().join(" ");
        if text.is_empty() { continue; }
        synced_lines.push(format!("[{}]{text}", format_lrc_timestamp(at as f64 / 1000.0)));
        plain_lines.push(text);
        if synced_lines.len() > 20000 { return Err("Terlalu banyak baris caption.".into()); }
    }
    if synced_lines.is_empty() { return Err("Tidak ada baris caption yang dapat diekstrak.".into()); }
    Ok(LyricsResult { plain_lyrics: plain_lines.join("\n"), synced_lyrics: synced_lines.join("\n"), instrumental: false, is_youtube: true })
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
    fn direct_caption_entry_requires_both_consents_before_transport() {
        let consent=crate::lyrics_gate::LyricsConsent::default();
        assert!(fetch_captions("synthetic title","synthetic artist",200.,"orig",&consent.snapshot()).is_err());
        consent.set(true,false);
        assert!(fetch_captions("synthetic title","synthetic artist",200.,"orig",&consent.snapshot()).is_err());
        consent.set(false,true);
        assert!(fetch_captions("synthetic title","synthetic artist",200.,"orig",&consent.snapshot()).is_err());
    }
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
    fn caption_word_spans_preserve_paragraph_timing() {
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
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("[00:01.00]Jurusan paling dibutuhin"));
        assert!(lines[0].ends_with("informatika"));
    }

    #[test]
    fn test_is_allowed_caption_host() {
        assert!(is_allowed_caption_host("https://www.youtube.com/api/timedtext?v=123"));
        assert!(is_allowed_caption_host("https://youtube.com/api/timedtext?v=123"));
        assert!(is_allowed_caption_host("https://rr1---sn-4g5ednls.googlevideo.com/videoplayback?expire=123"));
        assert!(is_allowed_caption_host("https://googlevideo.com/videoplayback"));
        assert!(!is_allowed_caption_host("http://www.youtube.com/api/timedtext"));
        assert!(!is_allowed_caption_host("https://attacker.com/timedtext"));
        assert!(!is_allowed_caption_host("https://youtube.com.attacker.com/api"));
        assert!(!is_allowed_caption_host("https://googlevideo.com.attacker.com/api"));
    }

    #[test]
    fn test_verify_candidate_match_duration_and_title() {
        let matching_details = serde_json::json!({
            "title": "Artist - Song Title (Official Music Video)",
            "lengthSeconds": "210"
        });
        assert!(verify_candidate_match(&matching_details, "Song Title", "Artist", 212.0).is_ok());
        assert!(verify_candidate_match(&serde_json::json!({"title":"Artist - Song Other", "lengthSeconds":"210"}), "Song Title", "Artist", 210.0).is_err());
        assert!(verify_candidate_match(&serde_json::json!({"title":"Song Title", "lengthSeconds":"210", "author":"Other Artist"}), "Song Title", "Artist", 210.0).is_err());
        assert!(verify_candidate_match(&serde_json::json!({"title":"Artist - Song Title"}), "Song Title", "Artist", 210.0).is_err());
        assert!(verify_candidate_match(&matching_details, "Song Title", "", 210.0).is_err());

        // Duration mismatch is rejected before accepting a caption candidate.
        let mismatch_duration = serde_json::json!({
            "title": "Artist - Song Title (Official Music Video)",
            "lengthSeconds": "300"
        });
        assert!(verify_candidate_match(&mismatch_duration, "Song Title", "Artist", 210.0).is_err());

        // Title mismatch
        let mismatch_title = serde_json::json!({
            "title": "Completely Different Random Video",
            "lengthSeconds": "210"
        });
        assert!(verify_candidate_match(&mismatch_title, "Song Title", "Artist", 210.0).is_err());
    }

    #[test]
    fn caption_long_paragraph_does_not_invent_timestamps() {
        let xml = r#"<timedtext format="3"><body>
<p t="2000" d="4000">Kalimat ini sengaja dibuat sangat panjang sekali agar terpicu pemotongan otomatis menjadi beberapa baris yang rapi.</p>
</body></timedtext>"#;

        let res = parse_timedtext(xml, 35, "Title", "Artist", 10.0).unwrap();
        let lines: Vec<&str> = res.synced_lyrics.lines().collect();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("[00:02.00]Kalimat ini"));
        assert!(lines[0].ends_with("rapi."));
    }
    #[test]
    fn session_detection_does_not_depend_on_other_windows() {
        assert!(!session_allows_captions("chrome.exe", "Song", "Album"));
        assert!(!session_allows_captions("spotify.exe", "YouTube Song", "YouTube"));
        assert!(session_allows_captions("youtube.app", "Song", "Album"));
        assert!(session_allows_captions("msedge.exe", "Song - YouTube", ""));
        assert_eq!(caption_srv3_url("https://www.youtube.com/api/timedtext?v=x&fmt=json3&sig=y"), "https://www.youtube.com/api/timedtext?v=x&sig=y&fmt=srv3");
        assert!(extract_video_ids("\"videoId\":\"短い").is_empty());
    }
}
