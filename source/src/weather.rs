//! Weather presentation and work policy; no I/O or render loop.
use crate::online::{ForecastHour,WeatherReport};
use chrono::{DateTime,Utc};
use std::time::Duration;

#[derive(Default)]
pub struct RefreshState { pub attempt:Option<std::time::Instant>, pub failed:bool }
impl RefreshState {
    pub fn cancel_pending(&mut self,pending:bool){if pending{self.attempt=None;}}
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Sky { Clear,Cloud,Rain,Night }
pub fn sky(code:u32,is_day:bool)->Sky {
    if !is_day { Sky::Night } else { match code { 0..=2=>Sky::Clear,51..=99=>Sky::Rain,_=>Sky::Cloud } }
}
impl Sky {
    pub fn colors(self)->(u32,u32) { match self { Self::Clear=>(0x23394b,0x5795b4),Self::Cloud=>(0x273846,0x56778e),Self::Rain=>(0x242f3e,0x45657c),Self::Night=>(0x171f30,0x304362) } }
}
pub fn upcoming_hours(report:&WeatherReport,now:DateTime<Utc>)->impl Iterator<Item=&ForecastHour> {
    report.hourly.iter().filter(move|hour|DateTime::parse_from_rfc3339(&hour.time).is_ok_and(|time|time>now)).take(4)
}
pub fn refresh_due(enabled:bool,visible:bool,busy:bool,failed:bool,elapsed:Option<Duration>)->bool {
    enabled&&visible&&!busy&&elapsed.is_none_or(|age|age>=Duration::from_secs(if failed{300}else{1800}))
}
pub fn summary(report:&WeatherReport,now:DateTime<Utc>,lang:&str)->String {
    let en=lang=="en";
    if let Some(hour)=upcoming_hours(report,now).find(|hour|hour.rain_chance.is_some_and(|chance|chance>=40)) {
        if let Ok(time)=DateTime::parse_from_rfc3339(&hour.time) { return if en{format!("Chance of rain around {}.",time.format("%H:%M"))}else{format!("Peluang hujan sekitar {}.",time.format("%H:%M"))}; }
    }
    let key=if upcoming_hours(report,now).next().is_none(){"weather.no_hourly"}else if upcoming_hours(report,now).all(|hour|hour.rain_chance.is_none()){"weather.no_rain_data"}else{"weather.low_rain"};
    crate::i18n::t(key,lang).into()
}
pub fn today(report:&WeatherReport,now:DateTime<Utc>)->Option<&crate::online::ForecastDay> {
    let date=DateTime::parse_from_rfc3339(&report.updated_at).ok().map(|time|now.with_timezone(time.offset()).format("%Y-%m-%d").to_string());
    report.daily.iter().find(|day|Some(day.date.as_str())==date.as_deref())
}
pub fn details(report:&WeatherReport,now:DateTime<Utc>,lang:&str)->String {
    let en=lang=="en";let feels=report.feels_like.round()as i32;
    if let Some(day)=today(report,now) {
        let(high,low)=(day.max.round()as i32,day.min.round()as i32);
        if en{format!("H:{high}° L:{low}° · Feels {feels}°")}else{format!("T:{high}° R:{low}° · Terasa {feels}°")}
    }else if en{format!("Feels {feels}°")}else{format!("Terasa {feels}°")}
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::online::WeatherReport;
    use chrono::{DateTime,Utc};
    use std::time::Duration;
    #[test] fn canceled_request_can_refresh_immediately_after_resume() {
        let mut state=RefreshState{attempt:Some(std::time::Instant::now()),failed:false};
        state.cancel_pending(false);
        assert!(!refresh_due(true,true,false,state.failed,state.attempt.map(|at|at.elapsed())));
        state.cancel_pending(true);
        assert!(refresh_due(true,true,false,state.failed,state.attempt.map(|at|at.elapsed())));
    }
    #[test] fn unknown_rain_probability_does_not_claim_low_rain() {
        let mut report=report();for hour in &mut report.hourly{hour.rain_chance=None;}
        let now=DateTime::parse_from_rfc3339("2026-10-28T16:42:00Z").unwrap().with_timezone(&Utc);
        assert_eq!(summary(&report,now,"en"),"Rain forecast unavailable.");
    }

    fn report() -> WeatherReport {
        serde_json::from_value(serde_json::json!({"temperature":32.,"feels_like":35.,"humidity":74,"wind_kph":12.,"weather_code":3,"is_day":true,"updated_at":"2026-10-28T23:42:00+07:00","daily":[],"hourly":[
            {"time":"invalid","temperature":99.,"weather_code":0,"rain_chance":0},
            {"time":"2026-10-28T23:00:00+07:00","temperature":32.,"weather_code":3,"rain_chance":10},
            {"time":"2026-10-29T00:00:00+07:00","temperature":28.,"weather_code":61,"rain_chance":75},
            {"time":"2026-10-29T01:00:00+07:00","temperature":27.,"weather_code":3,"rain_chance":20}]})).unwrap()
    }
    #[test] fn weather_palette_handles_day_night_and_precipitation() {
        assert_eq!(sky(0,true),Sky::Clear);assert_eq!(sky(3,true),Sky::Cloud);
        assert_eq!(sky(45,true),Sky::Cloud);assert_eq!(sky(61,true),Sky::Rain);
        assert_eq!(sky(95,true),Sky::Rain);assert_eq!(sky(0,false),Sky::Night);
        assert_eq!(sky(61,false),Sky::Night);
    }
    #[test] fn future_hours_cross_midnight_in_the_location_timezone() {
        let report=report();let now=DateTime::parse_from_rfc3339("2026-10-28T16:42:00Z").unwrap().with_timezone(&Utc);
        let hours:Vec<_>=upcoming_hours(&report,now).collect();
        assert_eq!(hours.len(),2);assert_eq!(hours[0].temperature,28.);assert!(hours[0].time.starts_with("2026-10-29T00:"));
        assert!(upcoming_hours(&report,now+chrono::Duration::hours(3)).next().is_none());
    }
    #[test] fn summary_follows_language_and_uses_only_future_rain() {
        let report=report();let now=DateTime::parse_from_rfc3339("2026-10-28T16:42:00Z").unwrap().with_timezone(&Utc);
        assert_eq!(summary(&report,now,"id"),"Peluang hujan sekitar 00:00.");
        assert_eq!(summary(&report,now,"en"),"Chance of rain around 00:00.");
        assert_eq!(summary(&report,now+chrono::Duration::hours(3),"en"),"Hourly forecast unavailable.");
    }
    #[test] fn refresh_is_quiet_when_disabled_hidden_busy_or_recent() {
        for (enabled,visible,busy) in [(false,true,false),(true,false,false),(true,true,true)] {
            assert!(!refresh_due(enabled,visible,busy,false,None));
        }
        assert!(refresh_due(true,true,false,false,None));
        assert!(!refresh_due(true,true,false,false,Some(Duration::from_secs(1799))));
        assert!(refresh_due(true,true,false,false,Some(Duration::from_secs(1800))));
        assert!(!refresh_due(true,true,false,true,Some(Duration::from_secs(299))));
        assert!(refresh_due(true,true,false,true,Some(Duration::from_secs(300))));
    }
    #[test] fn missing_daily_values_are_not_invented_and_details_are_bilingual() {
        let mut report=report();let now=DateTime::parse_from_rfc3339("2026-10-28T16:42:00Z").unwrap().with_timezone(&Utc);
        assert_eq!(details(&report,now,"id"),"Terasa 35°");assert_eq!(details(&report,now,"en"),"Feels 35°");
        report.daily.push(crate::online::ForecastDay{date:"2026-10-28".into(),min:26.,max:32.,weather_code:3,rain_chance:Some(75)});
        assert_eq!(details(&report,now,"id"),"T:32° R:26° · Terasa 35°");assert_eq!(details(&report,now,"en"),"H:32° L:26° · Feels 35°");
        assert_eq!(details(&report,now+chrono::Duration::hours(1),"en"),"Feels 35°");
    }
}
