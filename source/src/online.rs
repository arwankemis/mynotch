//! Opt-in online adapters. Construction is passive; each call is an explicit worker job.
use crate::{ical::{self, CalendarEvent, CalendarParseResult}, net};
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::HashMap, fs, io::Write, path::{Path, PathBuf}, sync::{Arc, Mutex}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct PrayerSchedule{pub city:String,pub date:String,pub times:Vec<String>}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForecastDay { pub date: String, pub min: f64, pub max: f64, pub weather_code: u32, pub rain_chance: u32 }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForecastHour { pub time: String, pub temperature: f64, pub weather_code: u32, pub rain_chance: u32 }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeatherReport {
    pub temperature: f64,
    pub feels_like: f64,
    pub humidity: u32,
    pub wind_kph: f64,
    pub weather_code: u32,
    pub is_day: bool,
    pub updated_at: String,
    pub daily: Vec<ForecastDay>,
    pub hourly: Vec<ForecastHour>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LyricsResult { pub plain_lyrics: String, pub synced_lyrics: String, pub instrumental: bool }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage { pub role: String, pub content: String }

#[derive(Serialize, Deserialize)]
struct DiskCache<T> { schema: u32, key: String, stored_at: u64, data: T }
#[derive(Clone)]
struct CachedAgenda { fetched: Instant, data: CalendarParseResult }

#[derive(Clone)]
pub struct OnlineService {
    root: PathBuf,
    agenda_cache: Arc<Mutex<HashMap<String, CachedAgenda>>>,
}

impl OnlineService {
    pub fn new(root: PathBuf) -> Self { Self { root, agenda_cache: Arc::new(Mutex::new(HashMap::new())) } }

    pub fn prayers(&self,city:&str,date:&str)->Result<PrayerSchedule,String>{
        if !["Jakarta","Bandung","Surabaya"].contains(&city)||chrono::NaiveDate::parse_from_str(date,"%Y-%m-%d").is_err(){return Err("Kota atau tanggal tidak valid.".into())}
        let key=format!("v3:{city}:{date}");
        if let Some(data)=self.read_cache::<PrayerSchedule>("prayers",&key,7*86400){if data.city==city&&data.date==date&&data.times.len()==5&&data.times.iter().all(|t|chrono::NaiveTime::parse_from_str(t,"%H:%M").is_ok())&&data.times.windows(2).all(|w|w[0]<w[1]){return Ok(data)}}
        let locations=json_response(&net::get(&format!("https://api.myquran.com/v3/sholat/kabkota/cari/{}",encode(city)))?)?;
        let wanted=format!("KOTA {}",city.to_uppercase());
        let id=locations["data"].as_array().and_then(|a|a.iter().find(|v|v["lokasi"].as_str()==Some(&wanted))).and_then(|v|v["id"].as_str()).ok_or("Kota belum ditemukan pada layanan jadwal.")?;
        if id.len()!=32||!id.bytes().all(|b|b.is_ascii_hexdigit()){return Err("ID kota layanan tidak valid.".into())}
        let bytes=net::get(&format!("https://api.myquran.com/v3/sholat/jadwal/{id}/{date}"))?;
        let schedule=parse_prayers(&bytes,city,date)?;net::check_cancel()?;let _=self.write_cache("prayers",&key,&schedule);Ok(schedule)
    }

    pub fn weather(&self, latitude: f64, longitude: f64) -> Result<WeatherReport, String> {
        net::check_cancel()?;
        if !latitude.is_finite() || !longitude.is_finite() || !(-90.0..=90.0).contains(&latitude) || !(-180.0..=180.0).contains(&longitude) { return Err("Koordinat cuaca tidak valid.".into()); }
        let key = format!("{latitude:.4},{longitude:.4}");
        if let Some(report) = self.read_cache("weather", &key, 30 * 60) { return Ok(report); }
        let url = format!("https://api.open-meteo.com/v1/forecast?latitude={latitude:.4}&longitude={longitude:.4}&current=temperature_2m,relative_humidity_2m,apparent_temperature,is_day,weather_code,wind_speed_10m&hourly=temperature_2m,precipitation_probability,weather_code&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max&temperature_unit=celsius&wind_speed_unit=kmh&timezone=auto&forecast_days=7&timeformat=unixtime");
        let body = net::get(&url)?;
        let report = parse_weather(&body)?;
        net::check_cancel()?;
        let _ = self.write_cache("weather", &key, &report);
        Ok(report)
    }

    pub fn lyrics(&self, title: &str, artist: &str, album: &str, duration: f64) -> Result<LyricsResult, String> {
        net::check_cancel()?;
        if title.trim().is_empty() || artist.trim().is_empty() { return Ok(LyricsResult::default()); }
        if [title, artist, album].iter().any(|v| v.len() > 4096) || !duration.is_finite() || !(0.0..=86400.0).contains(&duration) { return Err("Metadata lagu tidak valid.".into()); }
        let duration = duration.round() as u64;
        let key = serde_json::to_string(&(title.trim(), artist.trim(), album.trim(), duration)).map_err(|_| "Metadata lagu tidak valid.")?;
        if let Some(result) = self.read_cache::<LyricsResult>("lyrics", &key, 7 * 86400) {
            // A negative lookup has a shorter TTL, so newly published lyrics can appear.
            if result.instrumental || !result.plain_lyrics.is_empty() || !result.synced_lyrics.is_empty() || self.read_cache::<LyricsResult>("lyrics", &key, 30 * 60).is_some() { return Ok(result); }
        }
        let url = format!("https://lrclib.net/api/get?track_name={}&artist_name={}&album_name={}&duration={duration}", encode(title.trim()), encode(artist.trim()), encode(album.trim()));
        let result = match net::get(&url) {
            Ok(body) => parse_lyrics(&body)?,
            Err(error) if error == "Layanan mengembalikan HTTP 404." => LyricsResult::default(),
            Err(error) => return Err(error),
        };
        net::check_cancel()?;let _ = self.write_cache("lyrics", &key, &result);
        Ok(result)
    }

    /// Strict facade: unsupported calendar semantics produce an explicit error rather than an apparently complete agenda.
    pub fn agenda(&self, feed_url: &str) -> Result<Vec<CalendarEvent>, String> {
        let result = self.agenda_with_warnings(feed_url)?;
        if !result.warnings.is_empty() { return Err(format!("Feed kalender belum dapat ditampilkan lengkap: {}", result.warnings.iter().take(3).cloned().collect::<Vec<_>>().join(" "))); }
        Ok(result.events)
    }

    pub fn agenda_with_warnings(&self, feed_url: &str) -> Result<CalendarParseResult, String> {
        net::check_cancel()?;
        if !feed_url.starts_with("https://") || feed_url.len() > 16384 { return Err("URL feed kalender harus HTTPS.".into()); }
        {
            let cache = self.agenda_cache.lock().map_err(|_| "Cache agenda tidak tersedia.")?;
            if let Some(entry) = cache.get(feed_url).filter(|entry| entry.fetched.elapsed() < Duration::from_secs(15 * 60)) { return Ok(entry.data.clone()); }
        }
        let body = net::get(feed_url)?;
        if body.len() > 2 * 1024 * 1024 { return Err("Feed kalender melebihi batas 2 MiB.".into()); }
        let text = std::str::from_utf8(&body).map_err(|_| "Feed kalender bukan UTF-8 yang valid.")?;
        let result = ical::parse(text)?;
        net::check_cancel()?;
        let mut cache = self.agenda_cache.lock().map_err(|_| "Cache agenda tidak tersedia.")?;
        // Agenda titles and private feed URLs remain in memory; no plaintext agenda cache is written.
        cache.retain(|_, entry| entry.fetched.elapsed() < Duration::from_secs(15 * 60));
        if cache.len() >= 8 { cache.clear(); }
        cache.insert(feed_url.to_owned(), CachedAgenda { fetched: Instant::now(), data: result.clone() });
        Ok(result)
    }

    pub fn chat(&self, provider: &str, endpoint: &str, model: &str, key: &str, messages: &[ChatMessage]) -> Result<String, String> {
        let request = chat_request(provider, endpoint, model, key, messages)?;
        let header_refs = request.headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>();
        let body = net::request("POST", &request.url, &header_refs, &request.body)?;
        if body.len() > 1024 * 1024 { return Err("Respons AI melebihi batas 1 MiB.".into()); }
        parse_chat_response(request.kind, &body)
    }

    fn read_cache<T: DeserializeOwned>(&self, kind: &str, key: &str, ttl_seconds: u64) -> Option<T> {
        let path = cache_path(&self.root, kind, key);
        let metadata = fs::metadata(&path).ok()?;
        if metadata.len() > net::MAX_RESPONSE_BYTES as u64 { return None; }
        let bytes = fs::read(path).ok()?;
        let entry = serde_json::from_slice::<DiskCache<T>>(&bytes).ok()?;
        if entry.schema != 1 || entry.key != key || !fresh(entry.stored_at, now_seconds(), ttl_seconds) { return None; }
        Some(entry.data)
    }
    fn write_cache<T: Serialize>(&self, kind: &str, key: &str, data: &T) -> Result<(), String> {
        let path = cache_path(&self.root, kind, key);
        fs::create_dir_all(path.parent().expect("cache parent")).map_err(|_| "Cache belum dapat disimpan.")?;
        let bytes = serde_json::to_vec(&DiskCache { schema: 1, key: key.to_owned(), stored_at: now_seconds(), data }).map_err(|_| "Cache tidak dapat diserialisasi.")?;
        if bytes.len() > net::MAX_RESPONSE_BYTES { return Err("Cache melebihi batas ukuran.".into()); }
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let temporary = path.with_extension(format!("{}.{}.tmp", std::process::id(), nonce));
        let result = (|| {
            let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
            file.write_all(&bytes)?; file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &path)
        })();
        if result.is_err() { let _ = fs::remove_file(&temporary); return Err("Cache belum dapat disimpan.".into()); }
        Ok(())
    }
}

fn now_seconds() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }
fn fresh(saved: u64, now: u64, ttl: u64) -> bool { saved <= now && now - saved < ttl }
fn cache_path(root: &Path, kind: &str, key: &str) -> PathBuf {
    // This is only a filename selector, not a security hash. Full keys are compared after loading.
    let hash = key.as_bytes().iter().fold(0xcbf29ce484222325_u64, |state, byte| (state ^ u64::from(*byte)).wrapping_mul(0x100000001b3));
    root.join("cache").join(format!("{kind}-{hash:016x}.json"))
}
pub fn encode(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') { output.push(byte as char); }
        else { use std::fmt::Write; let _ = write!(output, "%{byte:02X}"); }
    }
    output
}
fn json_response(body: &[u8]) -> Result<Value, String> { serde_json::from_slice(body).map_err(|_| "Respons layanan bukan JSON yang valid.".into()) }
fn finite(value: &Value) -> Option<f64> { value.as_f64().filter(|number| number.is_finite()) }
fn integer(value: &Value) -> u32 { value.as_u64().unwrap_or(0).min(u32::MAX as u64) as u32 }
fn timestamp(value: &Value, offset: FixedOffset) -> Option<String> { DateTime::<Utc>::from_timestamp(value.as_i64()?, 0).map(|time| time.with_timezone(&offset).to_rfc3339()) }

fn parse_weather(body: &[u8]) -> Result<WeatherReport, String> {
    let value = json_response(body)?;
    let current = value.get("current").ok_or("Data cuaca saat ini tidak tersedia.")?;
    let offset = value.get("utc_offset_seconds").and_then(Value::as_i64).and_then(|seconds| i32::try_from(seconds).ok()).and_then(FixedOffset::east_opt).ok_or("Zona waktu cuaca tidak valid.")?;
    let temperature = finite(&current["temperature_2m"]).ok_or("Data suhu tidak tersedia.")?;
    let feels_like = finite(&current["apparent_temperature"]).unwrap_or(temperature);
    let updated_at = timestamp(&current["time"], offset).ok_or("Timestamp cuaca tidak valid.")?;
    let mut daily = Vec::new();
    let data = &value["daily"];
    if let Some(times) = data["time"].as_array() {
        for (index, time) in times.iter().take(7).enumerate() {
            let Some(date) = timestamp(time, offset).and_then(|s| s.get(..10).map(str::to_owned)) else { continue; };
            let (Some(min), Some(max)) = (finite(&data["temperature_2m_min"][index]), finite(&data["temperature_2m_max"][index])) else { continue; };
            daily.push(ForecastDay { date, min, max, weather_code: integer(&data["weather_code"][index]), rain_chance: integer(&data["precipitation_probability_max"][index]).min(100) });
        }
    }
    let mut hourly = Vec::new();
    let data = &value["hourly"];
    let now = current["time"].as_i64().unwrap_or(0);
    if let Some(times) = data["time"].as_array() {
        for (index, time) in times.iter().take(7 * 24).enumerate() {
            if time.as_i64().unwrap_or(0) < now - 3600 { continue; }
            let (Some(time), Some(temperature)) = (timestamp(time, offset), finite(&data["temperature_2m"][index])) else { continue; };
            hourly.push(ForecastHour { time, temperature, weather_code: integer(&data["weather_code"][index]), rain_chance: integer(&data["precipitation_probability"][index]).min(100) });
            if hourly.len() >= 24 { break; }
        }
    }
    Ok(WeatherReport { temperature, feels_like, humidity: integer(&current["relative_humidity_2m"]).min(100), wind_kph: finite(&current["wind_speed_10m"]).unwrap_or(0.), weather_code: integer(&current["weather_code"]), is_day: current["is_day"].as_u64() == Some(1), updated_at, daily, hourly })
}

fn parse_lyrics(body: &[u8]) -> Result<LyricsResult, String> {
    let value = json_response(body)?;
    if !value.is_object() { return Err("Respons lirik tidak valid.".into()); }
    let result = LyricsResult { plain_lyrics: value["plainLyrics"].as_str().unwrap_or("").to_owned(), synced_lyrics: value["syncedLyrics"].as_str().unwrap_or("").to_owned(), instrumental: value["instrumental"].as_bool().unwrap_or(false) };
    if result.plain_lyrics.len() > 1024 * 1024 || result.synced_lyrics.len() > 1024 * 1024 { return Err("Lirik melebihi batas ukuran.".into()); }
    Ok(result)
}

#[derive(Clone, Copy, Debug)]
enum ChatKind { OpenAi, Gemini, Anthropic }
struct ChatRequest { kind: ChatKind, url: String, headers: Vec<(String, String)>, body: Vec<u8> }
fn chat_request(provider: &str, endpoint: &str, model: &str, key: &str, messages: &[ChatMessage]) -> Result<ChatRequest, String> {
    if model.trim().is_empty() || model.len() > 256 || key.trim().is_empty() || key.len() > 16384 || key.contains(['\r', '\n', '\0']) { return Err("Isi model dan API key yang valid di Pengaturan AI.".into()); }
    if messages.is_empty() || messages.len() > 80 || messages.iter().map(|m| m.content.len()).sum::<usize>() > 256 * 1024 { return Err("Percakapan kosong atau melebihi batas ukuran.".into()); }
    if messages.iter().any(|m| !matches!(m.role.as_str(), "system" | "developer" | "user" | "assistant")) { return Err("Role percakapan belum didukung.".into()); }
    let normalized = provider.to_ascii_lowercase();
    let kind = if normalized.contains("gemini") { ChatKind::Gemini } else if normalized.contains("claude") || normalized.contains("anthropic") { ChatKind::Anthropic } else if normalized.contains("openai") || normalized.contains("compatible") { ChatKind::OpenAi } else { return Err("Provider AI belum didukung.".into()); };
    let default = match kind { ChatKind::OpenAi => "https://api.openai.com/v1", ChatKind::Gemini => "https://generativelanguage.googleapis.com/v1beta", ChatKind::Anthropic => "https://api.anthropic.com/v1" };
    let base = if endpoint.trim().is_empty() { default } else { endpoint.trim().trim_end_matches('/') };
    if !base.starts_with("https://") || base.contains(['?', '#', '@', '\r', '\n', '\0']) || base.len() > 8192 { return Err("Endpoint AI harus berupa URL basis HTTPS tanpa query atau userinfo.".into()); }
    let system = messages.iter().filter(|m| matches!(m.role.as_str(), "system" | "developer")).map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n\n");
    let conversation = messages.iter().filter(|m| matches!(m.role.as_str(), "user" | "assistant")).collect::<Vec<_>>();
    if conversation.is_empty() { return Err("Percakapan harus memiliki pesan pengguna atau asisten.".into()); }
    let mut headers = vec![("Content-Type".into(), "application/json".into())];
    let (url, body) = match kind {
        ChatKind::OpenAi => {
            headers.push(("Authorization".into(), format!("Bearer {}", key.trim())));
            let mut body = json!({"model":model.trim(),"messages":messages,"stream":false});
            // Native OpenAI reasoning models use this parameter; other compatible servers commonly use max_tokens.
            body[if base.starts_with("https://api.openai.com/") { "max_completion_tokens" } else { "max_tokens" }] = json!(2048);
            (append_endpoint(base, "chat/completions"), body)
        }
        ChatKind::Gemini => {
            headers.push(("x-goog-api-key".into(), key.trim().into()));
            let model = model.trim().strip_prefix("models/").unwrap_or(model.trim());
            if !model.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':')) { return Err("Nama model Gemini tidak valid.".into()); }
            let mut body = json!({"contents":conversation.iter().map(|m|json!({"role":if m.role=="assistant" {"model"}else{"user"},"parts":[{"text":m.content}]})).collect::<Vec<_>>(),"generationConfig":{"maxOutputTokens":2048}});
            if !system.is_empty() { body["systemInstruction"] = json!({"parts":[{"text":system}]}); }
            (format!("{base}/models/{}:generateContent", encode(model)), body)
        }
        ChatKind::Anthropic => {
            headers.push(("x-api-key".into(), key.trim().into()));
            headers.push(("anthropic-version".into(), "2023-06-01".into()));
            let mut body = json!({"model":model.trim(),"max_tokens":2048,"messages":conversation.iter().map(|m|json!({"role":m.role,"content":m.content})).collect::<Vec<_>>()});
            if !system.is_empty() { body["system"] = json!(system); }
            (append_endpoint(base, "messages"), body)
        }
    };
    let body = serde_json::to_vec(&body).map_err(|_| "Request AI tidak dapat dibuat.")?;
    Ok(ChatRequest { kind, url, headers, body })
}
fn append_endpoint(base: &str, suffix: &str) -> String { if base.ends_with(&format!("/{suffix}")) { base.to_owned() } else { format!("{base}/{suffix}") } }
fn text_parts(value: &Value) -> String { value.as_array().map(|parts| parts.iter().filter_map(|p|p["text"].as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default() }
fn parse_chat_response(kind: ChatKind, body: &[u8]) -> Result<String, String> {
    let value = json_response(body)?;
    let text = match kind {
        ChatKind::OpenAi => value["choices"][0]["message"]["content"].as_str().map(str::to_owned).unwrap_or_else(|| text_parts(&value["choices"][0]["message"]["content"])),
        ChatKind::Gemini => text_parts(&value["candidates"][0]["content"]["parts"]),
        ChatKind::Anthropic => text_parts(&value["content"]),
    };
    if text.trim().is_empty() { return Err("Provider AI tidak mengembalikan jawaban teks; periksa model atau kebijakan provider.".into()); }
    Ok(text)
}

pub fn weather_label(code: u32) -> &'static str {
    match code { 0 => "Cerah", 1 | 2 => "Cerah berawan", 3 => "Berawan", 45 | 48 => "Berkabut", 51 | 53 | 55 | 56 | 57 => "Gerimis", 61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => "Hujan", 71 | 73 | 75 | 77 | 85 | 86 => "Salju", 95 | 96 | 99 => "Badai petir", _ => "Cuaca" }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ttl_and_cache_identity_are_bounded() {
        assert!(fresh(1000, 1029, 30)); assert!(!fresh(1000, 1030, 30)); assert!(!fresh(1001, 1000, 30));
        let root = std::env::temp_dir().join(format!("notch-cache-test-{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let service = OnlineService::new(root.clone());
        service.write_cache("lyrics", "fixture", &LyricsResult { plain_lyrics:"hello".into(),synced_lyrics:"".into(),instrumental:false }).unwrap();
        assert_eq!(service.read_cache::<LyricsResult>("lyrics","fixture",30).unwrap().plain_lyrics,"hello");
        assert!(service.read_cache::<LyricsResult>("lyrics","other",30).is_none());
        let path=cache_path(&root,"lyrics","fixture");fs::write(path,"invalid JSON").unwrap();assert!(service.read_cache::<LyricsResult>("lyrics","fixture",30).is_none());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn query_encoding_preserves_metadata_boundaries() { assert_eq!(encode("A&B / C"),"A%26B%20%2F%20C");assert_eq!(encode("é"),"%C3%A9"); }
    #[test]
    fn provider_requests_keep_keys_out_of_url_and_preserve_system_messages() {
        let messages=vec![ChatMessage{role:"system".into(),content:"Be clear".into()},ChatMessage{role:"user".into(),content:"Hello".into()}];
        let request=chat_request("Gemini","","gemini-example","fixture-secret",&messages).unwrap();
        assert!(!request.url.contains("fixture-secret")); assert!(request.headers.iter().any(|(k,v)|k=="x-goog-api-key"&&v=="fixture-secret"));
        let body:Value=serde_json::from_slice(&request.body).unwrap();assert_eq!(body["systemInstruction"]["parts"][0]["text"],"Be clear");
        let request=chat_request("Anthropic","","claude-example","fixture-secret",&messages).unwrap();let body:Value=serde_json::from_slice(&request.body).unwrap();assert_eq!(body["system"],"Be clear");assert_eq!(body["max_tokens"],2048);
        let request=chat_request("OpenAI compatible","https://api.openai.com/v1","model-example","fixture-secret",&messages).unwrap();let body:Value=serde_json::from_slice(&request.body).unwrap();assert_eq!(body["max_completion_tokens"],2048);
    }
    #[test]
    fn parses_provider_text_without_live_calls() {
        assert_eq!(parse_chat_response(ChatKind::OpenAi,br#"{"choices":[{"message":{"content":"Hello"}}]}"#).unwrap(),"Hello");
        assert_eq!(parse_chat_response(ChatKind::Gemini,br#"{"candidates":[{"content":{"parts":[{"text":"Hello"}]}}]}"#).unwrap(),"Hello");
        assert_eq!(parse_chat_response(ChatKind::Anthropic,br#"{"content":[{"type":"text","text":"Hello"}]}"#).unwrap(),"Hello");
    }
    #[test]
    fn parses_weather_dates_in_response_offset() {
        let fixture=br#"{"utc_offset_seconds":25200,"current":{"time":1792796400,"temperature_2m":30,"apparent_temperature":33,"relative_humidity_2m":70,"wind_speed_10m":5,"weather_code":3,"is_day":1},"daily":{"time":[1792774800],"temperature_2m_min":[25],"temperature_2m_max":[32],"weather_code":[61],"precipitation_probability_max":[80]},"hourly":{"time":[1792796400],"temperature_2m":[30],"weather_code":[3],"precipitation_probability":[20]}}"#;
        let weather=parse_weather(fixture).unwrap();assert_eq!(weather.temperature,30.);assert!(weather.updated_at.ends_with("+07:00"));assert_eq!(weather.daily[0].rain_chance,80);assert_eq!(weather.hourly.len(),1);
    }
}

fn parse_prayers(bytes:&[u8],city:&str,date:&str)->Result<PrayerSchedule,String>{
    let value=json_response(bytes)?;
    let expected=format!("KOTA {}",city.to_uppercase());
    if value["status"].as_bool()!=Some(true)||value["data"]["kabko"].as_str()!=Some(&expected){return Err("Jadwal kota tidak sesuai.".into())}
    let day=&value["data"]["jadwal"][date];
    let times:Result<Vec<_>,String>=["subuh","dzuhur","ashar","maghrib","isya"].iter().map(|name|{let t=day[name].as_str().ok_or("Jadwal belum lengkap.")?;chrono::NaiveTime::parse_from_str(t,"%H:%M").map_err(|_|"Waktu sholat tidak valid.")?;Ok(t.to_string())}).collect();
    let times=times?;if times.windows(2).any(|w|w[0]>=w[1]){return Err("Urutan waktu layanan tidak valid.".into())}
    Ok(PrayerSchedule{city:city.into(),date:date.into(),times})
}
#[cfg(test)]mod prayer_tests{
    use super::*;
    #[test]fn schedule_rejects_wrong_city_date_and_invalid_times(){let data=br#"{"status":true,"data":{"kabko":"KOTA JAKARTA","jadwal":{"2026-10-26":{"subuh":"04:11","dzuhur":"11:40","ashar":"14:52","maghrib":"17:49","isya":"18:59"}}}}"#;assert_eq!(parse_prayers(data,"Jakarta","2026-10-26").unwrap().times[0],"04:11");assert!(parse_prayers(data,"Bandung","2026-10-26").is_err());assert!(parse_prayers(data,"Jakarta","2026-10-27").is_err());assert!(parse_prayers(&String::from_utf8_lossy(data).replace("04:11","25:00").into_bytes(),"Jakarta","2026-10-26").is_err());}
}
