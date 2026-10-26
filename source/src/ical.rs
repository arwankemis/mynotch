//! Bounded iCalendar subset, built independently from RFC 5545.
//! Unsupported recurrence/time zones are warnings, never guessed dates.
use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
    pub all_day: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CalendarParseResult {
    pub events: Vec<CalendarEvent>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
struct Property { name: String, params: HashMap<String, String>, value: String }
type Block = Vec<Property>;
#[derive(Clone, Copy)]
struct Stamp { local: NaiveDateTime, offset: FixedOffset, all_day: bool }
impl Stamp {
    fn instant(self) -> DateTime<FixedOffset> { self.offset.from_local_datetime(&self.local).single().expect("fixed offset") }
    fn at(self, date: NaiveDate) -> DateTime<FixedOffset> { self.offset.from_local_datetime(&date.and_time(self.local.time())).single().expect("fixed offset") }
}

pub fn parse(content: &str) -> Result<CalendarParseResult, String> {
    let now = Utc::now().with_timezone(&jakarta());
    parse_between(content, now - Duration::days(1), now + Duration::days(92))
}

pub fn parse_between(content: &str, from: DateTime<FixedOffset>, to: DateTime<FixedOffset>) -> Result<CalendarParseResult, String> {
    if content.len() > 2 * 1024 * 1024 { return Err("Feed kalender melebihi batas 2 MiB.".into()); }
    if to <= from || to - from > Duration::days(400) { return Err("Rentang agenda harus 1–400 hari.".into()); }
    if !content.contains("BEGIN:VCALENDAR") { return Err("Respons bukan feed iCalendar.".into()); }
    let blocks = read_blocks(content)?;
    let mut output = CalendarParseResult::default();
    let mut events = BTreeMap::<String, CalendarEvent>::new();
    let mut overrides: HashMap<String, Vec<&Block>> = HashMap::new();
    for block in &blocks {
        if field(block, "RECURRENCE-ID").is_some() {
            if let Some(uid) = field(block, "UID") { overrides.entry(unescape(&uid.value)).or_default().push(block); }
            else { warn(&mut output.warnings, "Pengecualian agenda tanpa UID dilewati."); }
        }
    }
    let mut candidates = 0_usize;
    for block in &blocks {
        if field(block, "RECURRENCE-ID").is_some() || cancelled(block) { continue; }
        // Commit a series only if its complete supported interpretation succeeded.
        let mut series = BTreeMap::new();
        match expand(block, &overrides, &mut series, from, to, &mut candidates) {
            Ok(()) => events.extend(series),
            Err(reason) => warn(&mut output.warnings, &reason),
        }
        if candidates >= 100000 || events.len() >= 2000 {
            warn(&mut output.warnings, "Kalender mencapai batas ekspansi; sebagian agenda belum ditampilkan.");
            break;
        }
    }
    output.events = events.into_values().collect();
    output.events.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.title.cmp(&b.title)));
    output.events.truncate(2000);
    Ok(output)
}

fn expand(block: &Block, overrides: &HashMap<String, Vec<&Block>>, output: &mut BTreeMap<String, CalendarEvent>, from: DateTime<FixedOffset>, to: DateTime<FixedOffset>, candidates: &mut usize) -> Result<(), String> {
    let start_property = field(block, "DTSTART").ok_or("Agenda tanpa DTSTART dilewati.")?;
    let first = parse_time(start_property, jakarta())?;
    let duration = event_duration(block, first)?;
    let title = unescape(&field(block, "SUMMARY").map(|p| p.value.as_str()).unwrap_or("Tanpa judul"));
    let uid = field(block, "UID").map(|p| unescape(&p.value)).unwrap_or_else(|| format!("{}:{title}", first.instant().timestamp()));
    if field(block, "RDATE").is_some() || field(block, "EXRULE").is_some() { return Err("Seri dengan RDATE/EXRULE belum didukung dan dilewati.".into()); }
    let rule = field(block, "RRULE").map(|p| Rule::parse(&p.value)).transpose()?;
    let mut excluded = HashSet::new();
    for property in block.iter().filter(|p| p.name == "EXDATE") {
        for value in property.value.split(',') {
            let mut item = property.clone(); item.value = value.to_owned();
            excluded.insert(parse_time(&item, first.offset)?.instant().timestamp());
        }
    }
    let mut replacements: HashMap<i64, &Block> = HashMap::new();
    if let Some(items) = overrides.get(&uid) {
        for item in items {
            let property = field(item, "RECURRENCE-ID").expect("override marker");
            if property.params.contains_key("RANGE") { return Err("Seri dengan RECURRENCE-ID RANGE belum didukung dan dilewati.".into()); }
            let stamp = parse_time(property, first.offset)?;
            replacements.insert(stamp.instant().timestamp(), item);
        }
    }
    let until = rule.as_ref().and_then(|r| r.values.get("UNTIL")).map(|value| {
        let stamp = parse_time(&Property { name: "UNTIL".into(), params: HashMap::new(), value: value.clone() }, first.offset)?;
        Ok::<_, String>(if stamp.all_day { stamp.instant() + Duration::days(1) - Duration::nanoseconds(1) } else { stamp.instant() })
    }).transpose()?;
    let count_limit = rule.as_ref().map(|r| r.count).unwrap_or(1);
    let final_date = to.with_timezone(&first.offset).date_naive().checked_add_signed(Duration::days(1)).ok_or("Tanggal kalender di luar rentang.")?;
    let mut date = first.local.date();
    let mut occurrences = 0_usize;
    while date <= final_date && occurrences < count_limit {
        *candidates += 1;
        if *candidates >= 100000 { return Err("Kalender mencapai batas 100.000 kandidat recurrence.".into()); }
        let matches = rule.as_ref().map(|r| r.matches(date, first.local.date())).unwrap_or(true);
        if date == first.local.date() || matches {
            let instant = first.at(date);
            if until.is_some_and(|end| instant > end) { break; }
            occurrences += 1;
            if !excluded.contains(&instant.timestamp()) {
                if let Some(replacement) = replacements.remove(&instant.timestamp()) {
                    add_replacement(replacement, &uid, instant.timestamp(), first.offset, output, from, to)?;
                } else {
                    add(output, CalendarEvent { id: event_id(&uid, instant.timestamp()), title: title.clone(), start: instant, end: instant + duration, all_day: first.all_day }, from, to);
                }
            }
        }
        if rule.is_none() || output.len() >= 2000 { break; }
        date = date.checked_add_signed(Duration::days(1)).ok_or("Tanggal recurrence di luar rentang.")?;
    }
    // An exception may have moved into our window from a later original date.
    for (original, replacement) in replacements { add_replacement(replacement, &uid, original, first.offset, output, from, to)?; }
    Ok(())
}

fn add(output: &mut BTreeMap<String, CalendarEvent>, event: CalendarEvent, from: DateTime<FixedOffset>, to: DateTime<FixedOffset>) {
    if event.start < to && (event.end > from || event.start >= from) { output.insert(event.id.clone(), event); }
}
fn add_replacement(block: &Block, uid: &str, original: i64, offset: FixedOffset, output: &mut BTreeMap<String, CalendarEvent>, from: DateTime<FixedOffset>, to: DateTime<FixedOffset>) -> Result<(), String> {
    if cancelled(block) { return Ok(()); }
    let start = parse_time(field(block, "DTSTART").ok_or("Pengecualian agenda tidak memiliki DTSTART.")?, offset)?;
    let duration = event_duration(block, start)?;
    add(output, CalendarEvent { id: event_id(uid, original), title: unescape(&field(block, "SUMMARY").map(|p| p.value.as_str()).unwrap_or("Tanpa judul")), start: start.instant(), end: start.instant() + duration, all_day: start.all_day }, from, to);
    Ok(())
}
fn event_id(uid: &str, original: i64) -> String { format!("{uid}/{original}") }
fn warn(warnings: &mut Vec<String>, text: &str) { if !warnings.iter().any(|s| s == text) { warnings.push(text.into()); } }
fn field<'a>(block: &'a Block, name: &str) -> Option<&'a Property> { block.iter().find(|p| p.name == name) }
fn cancelled(block: &Block) -> bool { field(block, "STATUS").is_some_and(|p| p.value.eq_ignore_ascii_case("CANCELLED")) }
fn jakarta() -> FixedOffset { FixedOffset::east_opt(7 * 3600).expect("WIB") }

fn parse_time(property: &Property, inherited: FixedOffset) -> Result<Stamp, String> {
    let value = property.value.trim();
    let all_day = property.params.get("VALUE").is_some_and(|s| s.eq_ignore_ascii_case("DATE")) || value.len() == 8;
    let offset = if value.ends_with('Z') { FixedOffset::east_opt(0).expect("UTC") }
    else if let Some(zone) = property.params.get("TZID") {
        match zone.as_str() {
            "Asia/Jakarta" | "Asia/Pontianak" | "SE Asia Standard Time" => jakarta(),
            "Asia/Makassar" | "Asia/Ujung_Pandang" | "Singapore Standard Time" => FixedOffset::east_opt(8 * 3600).expect("WITA"),
            "Asia/Jayapura" | "Tokyo Standard Time" => FixedOffset::east_opt(9 * 3600).expect("WIT"),
            "UTC" | "Etc/UTC" | "GMT" => FixedOffset::east_opt(0).expect("UTC"),
            _ => return Err("TZID di luar zona Indonesia/UTC belum didukung; agenda terkait dilewati.".into()),
        }
    } else { inherited };
    let local = if all_day {
        NaiveDate::parse_from_str(value, "%Y%m%d").ok().and_then(|d| d.and_hms_opt(0, 0, 0))
    } else {
        let raw = value.strip_suffix('Z').unwrap_or(value);
        NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M%S").or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M")).ok()
    }.ok_or("Tanggal iCalendar tidak valid; agenda terkait dilewati.")?;
    Ok(Stamp { local, offset, all_day })
}

fn event_duration(block: &Block, start: Stamp) -> Result<Duration, String> {
    let duration = if let Some(end) = field(block, "DTEND") {
        let end = parse_time(end, start.offset)?;
        if end.all_day != start.all_day { return Err("Jenis DTSTART/DTEND tidak cocok.".into()); }
        end.instant() - start.instant()
    } else if let Some(duration) = field(block, "DURATION") { parse_duration(&duration.value)? }
    else if start.all_day { Duration::days(1) } else { Duration::zero() };
    if duration < Duration::zero() || duration > Duration::days(366) || start.all_day && duration.num_seconds() % 86400 != 0 { return Err("Durasi agenda tidak valid.".into()); }
    Ok(duration)
}

fn parse_duration(value: &str) -> Result<Duration, String> {
    let value = value.strip_prefix('+').unwrap_or(value);
    let mut rest = value.strip_prefix('P').ok_or("DURATION kalender tidak didukung.")?;
    let mut seconds = 0_i64;
    let mut time = false;
    let mut any = false;
    while !rest.is_empty() {
        if let Some(next) = rest.strip_prefix('T') { if time { return Err("DURATION kalender tidak valid.".into()); } time = true; rest = next; continue; }
        let count = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
        if count == 0 || count >= rest.len() { return Err("DURATION kalender tidak valid.".into()); }
        let number = rest[..count].parse::<i64>().map_err(|_| "DURATION kalender terlalu besar.")?;
        let (unit, multiplier) = match rest.as_bytes()[count] {
            b'W' if !time => ('W', 604800), b'D' if !time => ('D', 86400),
            b'H' if time => ('H', 3600), b'M' if time => ('M', 60), b'S' if time => ('S', 1),
            _ => return Err("Unit DURATION kalender belum didukung.".into()),
        };
        seconds = seconds.checked_add(number.checked_mul(multiplier).ok_or("DURATION kalender terlalu besar.")?).ok_or("DURATION kalender terlalu besar.")?;
        if seconds > 366 * 86400 { return Err("DURATION kalender melebihi 366 hari.".into()); }
        any = true; rest = &rest[count + unit.len_utf8()..];
    }
    if !any { return Err("DURATION kalender kosong.".into()); }
    Ok(Duration::seconds(seconds))
}

struct Rule { values: HashMap<String, String>, frequency: String, interval: i64, count: usize, days: Option<Vec<Weekday>>, month_days: Option<Vec<i32>>, week_start: Weekday }
impl Rule {
    fn parse(value: &str) -> Result<Self, String> {
        let mut values = HashMap::new();
        for field in value.split(';') {
            let (key, value) = field.split_once('=').ok_or("RRULE tidak valid.")?;
            let key = key.to_ascii_uppercase();
            if !matches!(key.as_str(), "FREQ" | "INTERVAL" | "COUNT" | "UNTIL" | "BYDAY" | "BYMONTHDAY" | "WKST") || values.insert(key, value.to_ascii_uppercase()).is_some() { return Err("Seri memiliki opsi RRULE yang belum didukung; seri dilewati.".into()); }
        }
        let frequency = values.get("FREQ").ok_or("RRULE tidak memiliki FREQ.")?.to_owned();
        if !matches!(frequency.as_str(), "DAILY" | "WEEKLY" | "MONTHLY") { return Err("FREQ recurrence ini belum didukung; seri dilewati.".into()); }
        if values.contains_key("COUNT") && values.contains_key("UNTIL") { return Err("RRULE COUNT dan UNTIL tidak boleh digabung.".into()); }
        let interval = positive_number(values.get("INTERVAL"), 1, 366)? as i64;
        let count = positive_number(values.get("COUNT"), usize::MAX, 10000)?;
        let days = values.get("BYDAY").map(|v| v.split(',').map(weekday).collect::<Result<Vec<_>, _>>()).transpose()?;
        let month_days = values.get("BYMONTHDAY").map(|v| v.split(',').map(|v| {
            let number = v.parse::<i32>().map_err(|_| "BYMONTHDAY tidak valid.")?;
            if number == 0 || !(-31..=31).contains(&number) { return Err("BYMONTHDAY di luar rentang."); }
            Ok(number)
        }).collect::<Result<Vec<_>, &str>>()).transpose().map_err(str::to_owned)?;
        if frequency == "WEEKLY" && month_days.is_some() { return Err("BYMONTHDAY dengan WEEKLY belum didukung.".into()); }
        let week_start = values.get("WKST").map(|s| weekday(s)).transpose()?.unwrap_or(Weekday::Mon);
        Ok(Self { values, frequency, interval, count, days, month_days, week_start })
    }
    fn matches(&self, date: NaiveDate, first: NaiveDate) -> bool {
        if self.days.as_ref().is_some_and(|days| !days.contains(&date.weekday())) { return false; }
        if let Some(days) = &self.month_days {
            let next_month = if date.month() == 12 { NaiveDate::from_ymd_opt(date.year()+1, 1, 1) } else { NaiveDate::from_ymd_opt(date.year(), date.month()+1, 1) };
            let Some(last) = next_month.and_then(|d| d.pred_opt()) else { return false; };
            if !days.iter().any(|day| if *day > 0 { *day == date.day() as i32 } else { last.day() as i32 + *day + 1 == date.day() as i32 }) { return false; }
        }
        match self.frequency.as_str() {
            "DAILY" => (date - first).num_days() % self.interval == 0,
            "WEEKLY" => {
                let day_offset = |d: NaiveDate| (d.weekday().num_days_from_monday() as i64 - self.week_start.num_days_from_monday() as i64 + 7) % 7;
                let week_difference = (date - first).num_days() - day_offset(date) + day_offset(first);
                week_difference / 7 % self.interval == 0 && (self.days.is_some() || date.weekday() == first.weekday())
            }
            "MONTHLY" => {
                let months = (date.year() - first.year()) as i64 * 12 + date.month() as i64 - first.month() as i64;
                months % self.interval == 0 && (self.days.is_some() || self.month_days.is_some() || date.day() == first.day())
            }
            _ => false,
        }
    }
}

fn positive_number(value: Option<&String>, fallback: usize, maximum: usize) -> Result<usize, String> {
    let Some(value) = value else { return Ok(fallback); };
    let parsed = value.parse::<usize>().map_err(|_| "Angka RRULE tidak valid.")?;
    if parsed == 0 || parsed > maximum { return Err("Angka RRULE di luar batas.".into()); }
    Ok(parsed)
}
fn weekday(value: &str) -> Result<Weekday, String> {
    match value { "MO" => Ok(Weekday::Mon), "TU" => Ok(Weekday::Tue), "WE" => Ok(Weekday::Wed), "TH" => Ok(Weekday::Thu), "FR" => Ok(Weekday::Fri), "SA" => Ok(Weekday::Sat), "SU" => Ok(Weekday::Sun), _ => Err("Ordinal BYDAY atau hari recurrence belum didukung; seri dilewati.".into()) }
}

fn read_blocks(content: &str) -> Result<Vec<Block>, String> {
    let mut lines: Vec<String> = Vec::new();
    for line in content.lines() {
        let line = line.trim_end_matches('\r').trim_start_matches('\u{feff}');
        if line.len() > 32768 { return Err("Baris feed kalender terlalu panjang.".into()); }
        if (line.starts_with(' ') || line.starts_with('\t')) && !lines.is_empty() {
            let previous = lines.last_mut().expect("line exists"); previous.push_str(&line[1..]);
            if previous.len() > 65536 { return Err("Baris unfolded kalender terlalu panjang.".into()); }
        } else { lines.push(line.to_owned()); }
    }
    let mut blocks = Vec::new(); let mut block: Option<Block> = None; let mut nested = 0_usize;
    for line in lines {
        if line.eq_ignore_ascii_case("BEGIN:VEVENT") { block = Some(Vec::new()); nested = 0; continue; }
        if line.eq_ignore_ascii_case("END:VEVENT") {
            if let Some(current) = block.take() { blocks.push(current); }
            if blocks.len() > 10000 { return Err("Feed memiliki terlalu banyak agenda.".into()); }
            continue;
        }
        let Some(current) = block.as_mut() else { continue; };
        if line.to_ascii_uppercase().starts_with("BEGIN:") { nested += 1; continue; }
        if line.to_ascii_uppercase().starts_with("END:") { nested = nested.saturating_sub(1); continue; }
        if nested > 0 { continue; }
        let mut quoted = false;
        let Some(colon) = line.char_indices().find_map(|(i, c)| { if c == '"' { quoted = !quoted; } if c == ':' && !quoted { Some(i) } else { None } }) else { continue; };
        let mut header = line[..colon].split(';');
        let name = header.next().unwrap_or("").to_ascii_uppercase();
        let params = header.filter_map(|p| p.split_once('=')).map(|(key, value)| (key.to_ascii_uppercase(), value.trim_matches('"').to_owned())).collect();
        current.push(Property { name, params, value: line[colon+1..].to_owned() });
    }
    Ok(blocks)
}

fn unescape(value: &str) -> String {
    let mut output = String::new(); let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character == '\\' { if let Some(next) = chars.next() { output.push(if matches!(next, 'n' | 'N') { '\n' } else { next }); } }
        else { output.push(character); }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    fn window() -> (DateTime<FixedOffset>, DateTime<FixedOffset>) { (jakarta().with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(), jakarta().with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap()) }
    #[test]
    fn all_day_end_is_exclusive_and_folded_text_unescapes() {
        let (from,to) = window();
        let parsed = parse_between("BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:20261024\r\nDTEND;VALUE=DATE:20261025\r\nSUMMARY:Halo\\,\r\n  dunia\r\nEND:VEVENT\r\nEND:VCALENDAR",from,to).unwrap();
        assert!(parsed.warnings.is_empty()); assert_eq!(parsed.events.len(),1);
        assert_eq!(parsed.events[0].title,"Halo, dunia"); assert_eq!(parsed.events[0].end - parsed.events[0].start,Duration::days(1));
    }
    #[test]
    fn count_is_before_exdate_and_timezone_is_wib() {
        let (from,to) = window();
        let parsed = parse_between("BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:a\nDTSTART;TZID=Asia/Jakarta:20261024T090000\nDTEND;TZID=Asia/Jakarta:20261024T100000\nRRULE:FREQ=DAILY;COUNT=3\nEXDATE;TZID=Asia/Jakarta:20261025T090000\nSUMMARY:Test\nEND:VEVENT\nEND:VCALENDAR",from,to).unwrap();
        assert!(parsed.warnings.is_empty()); assert_eq!(parsed.events.len(),2); assert_eq!(parsed.events[1].start.day(),26); assert_eq!(parsed.events[0].start.offset().local_minus_utc(),7*3600);
    }
    #[test]
    fn weekly_byday_interval_and_unknown_timezone_are_explicit() {
        let (from,to) = window();
        let parsed = parse_between("BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:a\nDTSTART:20261005T020000Z\nRRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=4\nSUMMARY:Test\nEND:VEVENT\nBEGIN:VEVENT\nUID:b\nDTSTART;TZID=Custom/Unknown:20261010T020000\nEND:VEVENT\nEND:VCALENDAR",from,to).unwrap();
        assert_eq!(parsed.events.iter().map(|e|e.start.day()).collect::<Vec<_>>(),vec![5,7,19,21]); assert!(!parsed.warnings.is_empty());
    }
    #[test]
    fn monthly_31_skips_invalid_month_and_replacement_does_not_duplicate() {
        let from=jakarta().with_ymd_and_hms(2026,1,1,0,0,0).unwrap(); let to=from+Duration::days(100);
        let parsed=parse_between("BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:a\nDTSTART;TZID=Asia/Jakarta:20260131T090000\nRRULE:FREQ=MONTHLY;COUNT=2\nEND:VEVENT\nBEGIN:VEVENT\nUID:a\nRECURRENCE-ID;TZID=Asia/Jakarta:20260331T090000\nDTSTART;TZID=Asia/Jakarta:20260401T100000\nSUMMARY:Moved\nEND:VEVENT\nEND:VCALENDAR",from,to).unwrap();
        assert!(parsed.warnings.is_empty());assert_eq!(parsed.events.len(),2);assert_eq!(parsed.events[1].start.month(),4);assert_eq!(parsed.events[1].title,"Moved");
    }
    #[test]
    fn ordinal_rule_is_not_guessed() {
        let (from,to)=window();let parsed=parse_between("BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:a\nDTSTART:20261005T020000Z\nRRULE:FREQ=MONTHLY;BYDAY=1MO\nEND:VEVENT\nEND:VCALENDAR",from,to).unwrap();
        assert!(parsed.events.is_empty());assert!(!parsed.warnings.is_empty());
    }
}
