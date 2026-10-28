use super::*;
use crate::{i18n,weather};
use chrono::{DateTime,Utc};

impl Shell {
    pub(super) fn refresh_weather(&mut self){
        if !self.settings.enabled("Cuaca")||self.fullscreen||self.suspended||self.jobs.contains_key("Cuaca"){return}
        // Repeated clicks/scrolls must not spawn workers or reread the disk cache every frame.
        if self.weather_refresh.attempt.is_some_and(|at|at.elapsed()<Duration::from_secs(2)){return}
        self.weather_refresh.attempt=Some(Instant::now());
        let api=self.online.clone();let(lat,lon)=(self.settings.latitude,self.settings.longitude);
        self.job("Cuaca",move||Event::Weather(api.weather(lat,lon)));
    }
    pub(super) fn reset_weather_location(&mut self){
        self.cancel_job("Cuaca");self.weather=None;self.weather_updated=None;self.weather_refresh.attempt=None;self.weather_refresh.failed=false;self.weather_menu_open=false;
        self.drafts.retain(|(page,id),_|page!="Cuaca"||!(30..=32).contains(id));
    }
    pub(super) fn sync_weather_lifecycle(&mut self){
        if !self.settings.enabled("Cuaca") { self.reset_weather_location();self.weather_custom_open=false; }
    }
    pub(super) fn poll_weather(&mut self){
        let visible=self.tab=="Cuaca"&&!self.settings_open&&self.hover_stage>=1&&!self.fullscreen&&!self.suspended;
        if weather::refresh_due(self.settings.enabled("Cuaca"),visible,self.jobs.contains_key("Cuaca"),self.weather_refresh.failed,self.weather_refresh.attempt.map(|at|at.elapsed())){self.refresh_weather();}
    }
    pub(super) fn weather_page(&mut self,p:&mut Painter){unsafe{
        if !self.settings.enabled("Cuaca"){return}
        let lang=self.settings.language.clone();let t=|key|i18n::t(key,&lang);
        let (mid,bottom)=self.weather.as_ref().map(|w|weather::sky(w.weather_code,w.is_day)).unwrap_or(weather::Sky::Cloud).colors();
        p.weather_background(self.width,self.height,mid,bottom);
        if self.weather_custom_open {self.draw_weather_custom_modal(p);return}
        let city=if self.settings.weather_city.is_empty(){"Jakarta"}else{&self.settings.weather_city};
        p.outline_icon("navigation",18.,57.,10.,0xe8f0f7);
        p.text_ellipsis(city,34.,53.,184.,20.,13.,0xf5f8fc,true);
        p.outline_icon("down",220.,58.,10.,0xa9bfce);
        self.hit(16.,50.,222.,25.,Action::WeatherToggleMenu);
        let refresh=t("weather.refresh");let refresh_w=(p.measure_text(refresh,10.,false)+17.).min(82.);
        let refresh_x=254.-refresh_w;
        self.hit(refresh_x,211.,refresh_w,22.,Action::WeatherRefresh);
        p.outline_icon("undo",refresh_x,216.,11.,0xc7d9e7);
        p.text(refresh,refresh_x+16.,212.,refresh_w-16.,20.,10.,0xc7d9e7,false);
        if let Some(report)=self.weather.as_ref(){
            let now=Utc::now();
            let(_,condition)=i18n::weather_condition(report.weather_code,report.is_day,&lang);
            p.text_light(&format!("{:.0}°",report.temperature),18.,73.,156.,92.,72.,0xf5f8fc);
            p.weather_icon(report.weather_code,report.is_day,false,186.,104.,54.);
            let details=weather::details(report,now,&lang);let details_w=p.measure_text(&details,10.5,false);
            let condition_w=p.measure_text(condition,11.5,false).min((234.-details_w-8.).max(32.));
            p.text_ellipsis(condition,18.,175.,condition_w,20.,11.5,0xffffff,false);
            p.text_ellipsis(&details,18.+condition_w+8.,176.,234.-condition_w-8.,18.,10.5,0xd4e3ee,false);
            let time=DateTime::parse_from_rfc3339(&report.updated_at).ok().map(|time|time.format("%H:%M").to_string()).unwrap_or_else(||"—".into());
            let updated=format!("{} {time}",t(if self.weather_refresh.failed{"weather.stale"}else{"weather.updated"}));
            p.text_ellipsis(&updated,18.,212.,(refresh_x-26.).max(20.),20.,10.,0xb9cbd9,false);
            let x=266.;let w=(self.width-x-16.).max(100.);
            weather_card(p,x,54.,w,112.);
            p.text_ellipsis(&weather::summary(report,now,&lang),x+11.,64.,w-22.,18.,11.,0xe4eef7,false);
            let slot_w=(w-22.)/5.;
            draw_hour(p,x+11.,slot_w,t("weather.now"),report.weather_code,report.is_day,report.temperature);
            for(index,hour)in weather::upcoming_hours(report,now).enumerate(){
                let label=hour.time.get(11..13).unwrap_or("—");
                draw_hour(p,x+11.+(index+1)as f32*slot_w,slot_w,label,hour.weather_code,hour.is_day,hour.temperature);
            }
            // Missing future hours occupy honest empty slots rather than invented values.
            let filled=weather::upcoming_hours(report,now).count()+1;
            for index in filled..5 {p.text_centered("—",x+11.+index as f32*slot_w,102.,slot_w,38.,13.,0xb9cbd9,1.,false);}
            let metric_w=(w-16.)/3.;
            for(index,(icon,label,value))in [
                ("droplet",t("weather.humidity_short"),format!("{}%",report.humidity)),
                ("wind",t("weather.wind"),format!("{:.0} {}",report.wind_kph,t("weather.wind_unit"))),
                ("cloud",t("weather.rain_short"),weather::today(report,now).and_then(|day|day.rain_chance).map(|chance|format!("{chance}%")).unwrap_or_else(||"—".into())),
            ].into_iter().enumerate(){
                let mx=x+index as f32*(metric_w+8.);weather_card(p,mx,176.,metric_w,50.);
                p.outline_icon(icon,mx+9.,184.,10.,0xc7d9e7);
                p.text_ellipsis(label,mx+23.,181.,metric_w-29.,16.,10.,0xc7d9e7,false);
                p.text(&value,mx+9.,199.,metric_w-18.,21.,13.,0xffffff,false);
            }
        }else{
            let message=t(if self.jobs.contains_key("Cuaca"){"weather.loading"}else{"weather.unavailable"});
            p.weather_icon(3,true,false,105.,89.,54.);
            p.text(message,266.,94.,self.width-290.,70.,12.,0xd4e3ee,false);
        }
        if self.weather_menu_open {self.draw_weather_city_menu(p);}
    }}
    fn draw_weather_city_menu(&mut self,p:&mut Painter){unsafe{
        p.fill(16.,78.,206.,126.,12.,0x172737);p.stroke(16.,78.,206.,126.,12.,0x526c80,1.);
        for(index,(city,lat,lon))in [("Jakarta",-6.2088,106.8456),("Bandung",-6.9175,107.6191),("Surabaya",-7.2575,112.7521)].into_iter().enumerate(){
            let y=83.+index as f32*29.;
            if self.settings.weather_city==city{p.fill(21.,y,196.,27.,6.,0x31485b);}
            p.text(city,30.,y+4.,176.,20.,12.,0xffffff,false);
            self.hit(21.,y,196.,28.,Action::WeatherSetCity(city,lat,lon));
        }
        p.text(i18n::t("weather.custom",&self.settings.language),30.,174.,176.,22.,12.,0xffffff,false);
        self.hit(21.,170.,196.,29.,Action::WeatherToggleCustom);
    }}
    pub(super) fn draw_weather_custom_modal(&mut self,p:&mut Painter){unsafe{
        let lang=self.settings.language.clone();let t=|key|i18n::t(key,&lang);let x=(self.width-340.)/2.;let y=65.;
        p.fill_alpha(0.,44.,self.width,self.height-44.,0.,0x000000,0.55);
        p.fill(x,y,340.,226.,14.,0x182430);p.stroke(x,y,340.,226.,14.,0x405b70,1.);
        p.text_centered(t("weather.setup_title"),x,y+12.,340.,20.,13.,0xffffff,1.,true);
        for(index,key)in ["weather.setup_city","weather.setup_lat","weather.setup_lon"].into_iter().enumerate(){let yy=y+44.+index as f32*32.;p.text(t(key),x+20.,yy,90.,20.,11.,0xc7d9e7,false);p.stroke(x+110.,yy-2.,204.,24.,6.,0x526c80,1.);}
        p.text_centered(t("weather.location_hint"),x+10.,y+138.,320.,16.,10.,0xb9cbd9,1.,false);
        for(index,(label,action))in [(t("weather.cancel"),Action::WeatherToggleCustom),(t("weather.save_btn"),Action::WeatherSave)].into_iter().enumerate(){let xx=x+20.+index as f32*160.;p.fill(xx,y+170.,140.,34.,8.,if index==0{0x223344}else{0x3b5d78});p.text_centered(label,xx,y+170.,140.,34.,12.,0xffffff,1.,false);self.hit(xx,y+170.,140.,34.,action);}
    }}
    pub(super) fn draw_weather_stage1(&mut self,p:&mut Painter,right:f32){unsafe{
        let(code,is_day,temp)=self.weather.as_ref().map(|w|(w.weather_code,w.is_day,format!("{:.0}°",w.temperature))).unwrap_or((3,true,"—°".into()));

        let city=if self.settings.weather_city.is_empty(){"Jakarta"}else{&self.settings.weather_city};
        let info=format!("{temp} {city}");
        let time=Local::now().format("%H:%M").to_string();
        let info_x=self.draw_stage1_status(p,&info,&time,right+4.);
        p.weather_icon(code,is_day,true,info_x-28.,(self.height-20.)/2.,20.);
    }}
}
unsafe fn weather_card(p:&Painter,x:f32,y:f32,w:f32,h:f32){unsafe{p.fill_alpha(x,y,w,h,11.,0x142636,0.48);p.stroke(x,y,w,h,11.,0x516a7e,0.6);}}
unsafe fn draw_hour(p:&mut Painter,x:f32,w:f32,label:&str,code:u32,is_day:bool,temperature:f64){unsafe{
    p.text_centered(label,x,92.,w,16.,11.,0xcadbe8,1.,false);p.weather_icon(code,is_day,false,x+(w-26.)/2.,110.,26.);p.text_centered(&format!("{temperature:.0}°"),x,140.,w,17.,12.,0xffffff,1.,false);
}}
