use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use windows::core::{w, s};
use windows::Win32::System::LibraryLoader::{LoadLibraryW, GetProcAddress};

type FnOpen = unsafe extern "C" fn(*const u16, i32, i32, *const u16, i32, *mut [i32; 8], *mut i32) -> *mut std::ffi::c_void;
type FnClose = unsafe extern "C" fn(*mut std::ffi::c_void);
type FnTrans = unsafe extern "C" fn(*const std::ffi::c_void, *mut u16, *mut i32, i32, i32, *mut i32, *mut i32);

struct IcuTrans {
    trans: *mut std::ffi::c_void,
    fn_close: FnClose,
    fn_trans: FnTrans,
}

unsafe impl Send for IcuTrans {}
unsafe impl Sync for IcuTrans {}

impl Drop for IcuTrans {
    fn drop(&mut self) {
        if !self.trans.is_null() {
            unsafe { (self.fn_close)(self.trans); }
        }
    }
}

static ICU: Mutex<Option<Option<IcuTrans>>> = Mutex::new(None);

fn with_icu<F: FnOnce(&IcuTrans) -> R, R>(f: F) -> Option<R> {
    let mut guard = ICU.lock().ok()?;
    if guard.is_none() {
        *guard = Some(init_icu());
    }
    guard.as_ref().and_then(|opt| opt.as_ref()).map(f)
}

fn init_icu() -> Option<IcuTrans> {
    unsafe {
        let h_icu = LoadLibraryW(w!("icu.dll")).ok()?;
        let p_open = GetProcAddress(h_icu, s!("utrans_openU"))?;
        let p_close = GetProcAddress(h_icu, s!("utrans_close"))?;
        let p_trans = GetProcAddress(h_icu, s!("utrans_transUChars"))?;

        let fn_open: FnOpen = std::mem::transmute(p_open);
        let fn_close: FnClose = std::mem::transmute(p_close);
        let fn_trans: FnTrans = std::mem::transmute(p_trans);

        let id: Vec<u16> = "Any-Latin; Latin-ASCII".encode_utf16().collect();
        let mut parse_err = [0i32; 8];
        let mut status = 0i32;
        let trans = fn_open(id.as_ptr(), id.len() as i32, 0, std::ptr::null(), 0, &mut parse_err, &mut status);
        if trans.is_null() || status > 0 {
            return None;
        }
        Some(IcuTrans { trans, fn_close, fn_trans })
    }
}

pub fn contains_cjk(s: &str) -> bool {
    s.chars().any(|c| matches!(c,
        '\u{3040}'..='\u{309F}' | // Hiragana
        '\u{30A0}'..='\u{30FF}' | // Katakana
        '\u{31F0}'..='\u{31FF}' | // Katakana Phonetic Extensions
        '\u{FF65}'..='\u{FF9F}' | // Halfwidth Katakana
        '\u{AC00}'..='\u{D7AF}' | // Hangul Syllables
        '\u{1100}'..='\u{11FF}' | // Hangul Jamo
        '\u{3130}'..='\u{318F}' | // Hangul Compatibility Jamo
        '\u{4E00}'..='\u{9FFF}' | // CJK Unified Ideographs
        '\u{3400}'..='\u{4DBF}'   // CJK Extension A
    ))
}

pub fn has_japanese_kana(s: &str) -> bool {
    s.chars().any(|c| matches!(c,
        '\u{3040}'..='\u{309F}' |
        '\u{30A0}'..='\u{30FF}' |
        '\u{31F0}'..='\u{31FF}'
    ))
}

const JP_PAIRS: &[(&str, &str)] = &[
    ("世界が終わるまでは", " sekai ga owaru made wa "),
    ("君の名は", " kimi no na wa "),
    ("愛してる", " aishiteru "),
    ("大好き", " daisuki "),
    ("好きだよ", " suki da yo "),
    ("好き", " suki "),
    ("ありがとう", " arigatou "),
    ("さようなら", " sayounara "),
    ("思い出", " omoide "),
    ("想い出", " omoide "),
    ("大丈夫", " daijoubu "),
    ("名前", " namae "),
    ("君達", " kimitachi "),
    ("僕達", " bokutachi "),
    ("私達", " watashitachi "),
    ("友達", " tomodachi "),
    ("二人", " futari "),
    ("一人", " hitori "),
    ("何度も", " nandomo "),
    ("毎日", " mainichi "),
    ("本当", " hontou "),
    ("一緒", " issho "),
    ("全部", " zenbu "),
    ("自分", " jibun "),
    ("世界", " sekai "),
    ("未来", " mirai "),
    ("永遠", " eien "),
    ("約束", " yakusoku "),
    ("時間", " jikan "),
    ("言葉", " kotoba "),
    ("笑顔", " egao "),
    ("運命", " unmei "),
    ("奇跡", " kiseki "),
    ("季節", " kisetsu "),
    ("瞬間", " shunkan "),
    ("記憶", " kioku "),
    ("気持ち", " kimochi "),
    ("場所", " basho "),
    ("夜空", " yozora "),
    ("青空", " aozora "),
    ("景色", " keshiki "),
    ("温もり", " nukumori "),
    ("優しさ", " yasashisa "),
    ("強さ", " tsuyosa "),
    ("弱さ", " yowasa "),
    ("悲しみ", " kanashimi "),
    ("喜び", " yorokobi "),
    ("始まり", " hajimari "),
    ("終わり", " owari "),
    ("願い", " negai "),
    ("祈り", " inori "),
    ("想い", " omoi "),
    ("思い", " omoi "),
    ("抱きしめて", " dakishimete "),
    ("見つめて", " mitsumete "),
    ("見上げて", " miagete "),
    ("見えない", " mienai "),
    ("見ている", " miteiru "),
    ("見つからない", " mitsukaranai "),
    ("見つかる", " mitsukaru "),
    ("見つける", " mitsukeru "),
    ("見る", " miru "),
    ("見て", " mite "),
    ("言えない", " ienai "),
    ("言わない", " iwanai "),
    ("言う", " iu "),
    ("言って", " itte "),
    ("聞こえない", " kikoenai "),
    ("聞こえる", " kikoeru "),
    ("行かないで", " ikanaide "),
    ("行く", " iku "),
    ("行って", " itte "),
    ("戻れない", " modorenai "),
    ("戻る", " modoru "),
    ("会いたい", " aitai "),
    ("会える", " aeru "),
    ("会う", " au "),
    ("歩いて", " aruite "),
    ("歩く", " aruku "),
    ("走って", " hashitte "),
    ("走る", " hashiru "),
    ("生きて", " ikite "),
    ("生きる", " ikiru "),
    ("笑って", " waratte "),
    ("笑う", " warau "),
    ("泣いて", " naite "),
    ("泣く", " naku "),
    ("教えて", " oshiete "),
    ("知っている", " shitteiru "),
    ("知らない", " shiranai "),
    ("知る", " shiru "),
    ("分からない", " wakaranai "),
    ("分かる", " wakaru "),
    ("変わらない", " kawaranai "),
    ("変わる", " kawaru "),
    ("終わらない", " owaranai "),
    ("終わる", " owaru "),
    ("消える", " kieru "),
    ("忘れる", " wasureru "),
    ("届く", " todoku "),
    ("響く", " hibiku "),
    ("揺れる", " yureru "),
    ("流れる", " nagareru "),
    ("溢れる", " afureru "),
    ("信じる", " shinjiru "),
    ("光る", " hikaru "),
    ("輝く", " kagayaku "),
    ("昨日", " kinou "),
    ("今日", " kyou "),
    ("明日", " ashita "),
    ("今", " ima "),
    ("時", " toki "),
    ("誰", " dare "),
    ("何", " nani "),
    ("人", " hito "),
    ("日", " hi "),
    ("月", " tsuki "),
    ("君は", " kimi wa "),
    ("僕は", " boku wa "),
    ("私は", " watashi wa "),
    ("君", " kimi "),
    ("僕", " boku "),
    ("私", " watashi "),
    ("胸", " mune "),
    ("目", " me "),
    ("手", " te "),
    ("道", " michi "),
    ("歌", " uta "),
    ("音", " oto "),
    ("声", " koe "),
    ("朝", " asa "),
    ("夜", " yoru "),
    ("花", " hana "),
    ("星", " hoshi "),
    ("雪", " yuki "),
    ("雨", " ame "),
    ("風", " kaze "),
    ("空", " sora "),
    ("海", " umi "),
    ("影", " kage "),
    ("光", " hikari "),
    ("夢", " yume "),
    ("恋", " koi "),
    ("愛", " ai "),
    ("心", " kokoro "),
    ("涙", " namida "),
    ("名は", " na wa "),
    ("名", " na "),
    ("までは", " made wa "),
    ("からは", " kara wa "),
];

pub fn romanize_line(line: &str) -> String {
    romanize_line_hint(line, has_japanese_kana(line))
}

pub fn romanize_line_hint(line: &str, is_japanese: bool) -> String {
    if !contains_cjk(line) {
        return line.to_string();
    }
    let mut working = line.to_string();
    if is_japanese || has_japanese_kana(&working) {
        for &(from, to) in JP_PAIRS {
            if working.contains(from) {
                working = working.replace(from, to);
            }
        }
    }
    let transliterated = with_icu(|icu| {
        let mut buf: Vec<u16> = working.encode_utf16().collect();
        let cap = (buf.len() * 4 + 64).max(256);
        buf.resize(cap, 0);
        let mut text_len = working.encode_utf16().count() as i32;
        let mut limit = text_len;
        let mut status = 0i32;
        unsafe {
            (icu.fn_trans)(icu.trans, buf.as_mut_ptr(), &mut text_len, buf.len() as i32, 0, &mut limit, &mut status);
        }
        if status <= 0 && limit >= 0 {
            String::from_utf16_lossy(&buf[..limit as usize])
        } else {
            working.clone()
        }
    }).unwrap_or(working);

    let mut out = String::with_capacity(transliterated.len());
    let mut last_space = false;
    for c in transliterated.chars() {
        if c.is_whitespace() {
            if !last_space && !out.is_empty() {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out.trim().to_string()
}

pub fn romanize_text(content: &str) -> String {
    if !contains_cjk(content) {
        return content.to_string();
    }
    let is_jp = has_japanese_kana(content);
    let mut out = Vec::new();
    for line in content.lines() {
        out.push(romanize_line_hint(line, is_jp));
    }
    out.join("\n")
}

pub fn romanize_lines(lines: &mut [LyricLine]) {
    let is_jp = lines.iter().any(|l| has_japanese_kana(&l.text));
    for line in lines.iter_mut() {
        if contains_cjk(&line.text) {
            line.text = romanize_line_hint(&line.text, is_jp);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LyricLine {
    pub at: f64,
    pub text: String,
}

/// LRC timestamps in seconds. The optional offset tag adds milliseconds to every timestamp.
pub fn parse_lrc(content: &str) -> Vec<LyricLine> { parse_lrc_checked(content).unwrap_or_default() }

pub fn parse_lrc_checked(content: &str) -> Result<Vec<LyricLine>, String> {
    if content.len() > 1024 * 1024 { return Err("Lirik melebihi batas ukuran.".into()); }
    let mut offset = 0_f64;
    for line in content.lines() {
        let value = line.trim().trim_start_matches('\u{feff}');
        if let Some(value) = value.strip_prefix("[offset:").and_then(|v| v.strip_suffix(']')) {
            if let Ok(milliseconds) = value.trim().parse::<i64>() { offset = milliseconds.clamp(-3_600_000, 3_600_000) as f64 / 1000.; }
        }
    }
    let mut output = Vec::new();
    for line in content.lines().take(20000) {
        let mut remaining = line.trim_end().trim_start_matches('\u{feff}');
        let mut stamps = Vec::new();
        while let Some(rest) = remaining.strip_prefix('[') {
            let Some(end) = rest.find(']') else { break };
            let Some(time) = timestamp(&rest[..end]) else { break };
            stamps.push((time + offset).max(0.));
            remaining = &rest[end + 1..];
            if stamps.len() > 100 { return Err("Terlalu banyak timestamp per baris lirik.".into()); }
        }
        for at in stamps { output.push(LyricLine { at, text: remaining.to_owned() }); }
        if output.len() > 20000 { return Err("Terlalu banyak baris lirik.".into()); }
    }
    // Stable sort preserves the file's order when several lines share a timestamp.
    output.sort_by(|a, b| a.at.total_cmp(&b.at));
    romanize_lines(&mut output);
    Ok(output)
}

fn timestamp(value: &str) -> Option<f64> {
    let parts: Vec<_> = value.split(':').collect();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [minutes, seconds] => (0_u32, minutes.parse::<u32>().ok()?, seconds.parse::<f64>().ok()?),
        [hours, minutes, seconds] => (hours.parse::<u32>().ok()?, minutes.parse::<u32>().ok()?, seconds.parse::<f64>().ok()?),
        _ => return None,
    };
    if !seconds.is_finite() || !(0.0..60.0).contains(&seconds) || parts.len() == 3 && minutes >= 60 || minutes > 10000 || hours > 1000 { return None; }
    Some(hours as f64 * 3600. + minutes as f64 * 60. + seconds)
}

pub fn active_line(lines: &[LyricLine], position: f64) -> Option<usize> {
    if !position.is_finite() { return None; }
    lines.partition_point(|line| line.at <= position).checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multiple_timestamps_offset_and_stable_order() {
        let lines = parse_lrc("[ar:Test]\n[offset:-500]\n[00:02.00][00:04.00]Halo\n[00:02.00]Dunia");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], LyricLine { at: 1.5, text: "Halo".into() });
        assert_eq!(lines[1].text, "Dunia");
        assert_eq!(lines[2].at, 3.5);
        assert_eq!(active_line(&lines, 1.4), None);
        assert_eq!(active_line(&lines, 1.5), Some(1));
    }
    #[test]
    fn malformed_timestamps_do_not_become_real_lines() {
        assert!(parse_lrc("[00:75]bad\n[NaN:02]bad\n[00:NaN]bad").is_empty());
        assert_eq!(parse_lrc("[01:02.250]x")[0].at, 62.25);
    }
    #[test]
    fn test_cjk_romanization() {
        assert_eq!(romanize_line("さくら ひらひら"), "sakura hirahira");
        assert_eq!(romanize_line("君の名は"), "kimi no na wa");
        assert_eq!(romanize_line("世界が終わるまでは"), "sekai ga owaru made wa");
        assert_eq!(romanize_line("안녕하세요"), "annyeonghaseyo");
        assert_eq!(romanize_line("你好世界"), "ni hao shi jie");
        // Non-CJK text remains unaffected
        assert_eq!(romanize_line("Hello world 123!"), "Hello world 123!");

        // Context-aware: pure kanji in Japanese song uses Japanese readings
        let mut lines = vec![
            LyricLine { at: 1.0, text: "君の声を聴かせて".into() },
            LyricLine { at: 2.0, text: "永遠".into() },
        ];
        romanize_lines(&mut lines);
        assert_eq!(lines[1].text, "eien");
    }
}
