use super::*;
use crate::mod_script::{LuaHeadline, LuaRanked};
use render::MUTED;
use chrono::Datelike;

impl Shell {
    pub(super) fn draw_page(&mut self,p:&mut Painter){unsafe{
        match self.tab.as_str(){
            "Musik"=>self.music_page(p),
            "Kalender"=>self.calendar_page(p),
            "Reminder"=>self.reminder_page(p),
            "Clipboard"=>self.clipboard_page(p),
            "Catatan"=>self.notes_page(p),
            "Cuaca"=>self.weather_page(p),
            _=>{
                if !self.draw_mod_tab(p) {
                    let msg = crate::i18n::t("shell.select_module", &self.settings.language);
                    p.text(msg,24.,140.,620.,80.,18.,MUTED,false);
                }
            }
        }
    }}

    fn music_page(&mut self,p:&mut Painter){
        if self.lyric_mode{
            self.draw_mockup_lyrics(p);
        }else{
            self.draw_mockup_player(p);
        }
    }

    fn draw_mockup_player(&mut self,p:&mut Painter){unsafe{
        let lang = self.settings.language.clone();
        // 1. Track Content (Artwork 60x60 from y=44 to y=104)
        let art_x=24.;let art_y=44.;let art_size=60.;
        let key=if self.cover.is_some(){self.cover_key}else{0};
        if !p.cover(self.cover.as_ref(),key,art_x,art_y,art_size,false){
            p.fill(art_x,art_y,art_size,art_size,8.,0x16222b);
            p.text("♫",art_x+19.,art_y+13.,24.,30.,22.,0x8dc4dd,false);
        }

        // Title (bold 14px white)
        let default_title = crate::i18n::t("music.no_music", &lang);
        let title=if self.track.title.is_empty(){default_title}else{&self.track.title};
        p.text_ellipsis(title,94.,44.,self.width-138.,18.,14.,0xffffff,true);

        // Media source app icon (Spotify, etc) placed at far right of title row
        p.icon_media_app(self.width-28.,52.,&self.track.source_app,self.track.playing);

        // Artist (12px #9ea3a8)
        let default_artist = crate::i18n::t("music.no_artist", &lang);
        let artist=if self.track.artist.is_empty(){default_artist.to_string()}else{self.track.artist.clone()};
        p.text_ellipsis(&artist,94.,65.,self.width-138.,16.,12.,0x9ea3a8,false);

        // Album (11px #6e7681)
        let default_album = crate::i18n::t("music.no_album", &lang);
        let album=if self.track.album.is_empty(){default_album.to_string()}else{self.track.album.clone()};
        p.text_ellipsis(&album,94.,85.,self.width-138.,16.,11.,0x6e7681,false);

        // 2. Scrubber (At y=114, below the artwork)
        let position=self.media.as_ref().map(|m|m.position_seconds()).unwrap_or(self.track.position_seconds);
        let duration=self.track.duration_seconds;
        let ratio=if duration>0.{(position/duration).clamp(0.,1.)}else{0.};

        let scrub_y=114.;
        p.text(&time_label(position),24.,scrub_y,34.,16.,11.,0xffffff,false);
        let track_x=62.;let track_w=self.width-124.;
        let scrub_hov=self.track.can_seek&&self.pointer.is_some_and(|(px,py)|px>=track_x&&px<=track_x+track_w&&py>=scrub_y&&py<=scrub_y+18.);
        p.fill(track_x,scrub_y+6.,track_w,4.,2.,if scrub_hov{0x52605e}else{0x3e4948});
        if ratio>0.{
            p.fill(track_x,scrub_y+6.,track_w*ratio as f32,4.,2.,0xffffff);
        }
        if scrub_hov{
            let thumb_x=(track_x+track_w*ratio as f32-5.).clamp(track_x-2.,track_x+track_w-8.);
            p.fill(thumb_x,scrub_y+3.,10.,10.,5.,0xffffff);
        }
        if self.focused.as_ref()==Some(&Action::SeekPointer){
            p.stroke(track_x-2.,scrub_y+2.,track_w+4.,12.,4.,0xd5d5dc,1.);
        }
        if self.track.can_seek&&self.hit_enabled{
            self.hits.push(Hit{rect:render::rect(track_x,scrub_y,track_w,18.),action:Action::SeekPointer});
        }
        let remaining=(duration-position).max(0.);
        p.text(&format!("-{}",time_label(remaining)),self.width-58.,scrub_y,40.,16.,11.,0xffffff,false);

        // 3. Transport Controls (At y=138, below scrubber)
        let trans_y=138.;
        // Lyrics toggle quote button at x=24
        self.transport_btn(p,"LYRICS",24.,trans_y,28.,28.,Action::ToggleLyrics,self.lyric_mode,true);
        // Centered Previous / Play / Next
        let mid_x=self.width/2.;
        self.transport_btn(p,"⏮",mid_x-52.,trans_y,28.,28.,Action::Previous,false,self.track.can_previous);
        let play_symbol=if self.track.playing{"⏸"}else{"▶"};
        self.transport_btn(p,play_symbol,mid_x-14.,trans_y,28.,28.,Action::Play,false,self.track.can_toggle);
        self.transport_btn(p,"⏭",mid_x+24.,trans_y,28.,28.,Action::Next,false,self.track.can_next);
    }}

    fn draw_mockup_lyrics(&mut self,p:&mut Painter){unsafe{
        let lang = self.settings.language.clone();
        let w=self.width;
        // Left Half: Track Content (x = 24..230)
        let art_x=24.;let art_y=44.;let art_size=60.;
        let key=if self.cover.is_some(){self.cover_key}else{0};
        if !p.cover(self.cover.as_ref(),key,art_x,art_y,art_size,false){
            p.fill(art_x,art_y,art_size,art_size,8.,0x16222b);
            p.text("♫",art_x+19.,art_y+13.,24.,30.,22.,0x8dc4dd,false);
        }

        let default_title = crate::i18n::t("music.no_music", &lang);
        let title=if self.track.title.is_empty(){default_title}else{&self.track.title};
        p.text_ellipsis(title,94.,44.,120.,18.,14.,0xffffff,true);
        p.icon_media_app(224.,52.,&self.track.source_app,self.track.playing);

        let default_artist = crate::i18n::t("music.no_artist", &lang);
        let artist=if self.track.artist.is_empty(){default_artist.to_string()}else{self.track.artist.clone()};
        p.text_ellipsis(&artist,94.,65.,126.,16.,12.,0x9ea3a8,false);

        let default_album = crate::i18n::t("music.single_album", &lang);
        let album=if self.track.album.is_empty(){default_album.to_string()}else{self.track.album.clone()};
        p.text_ellipsis(&album,94.,85.,126.,16.,11.,0x6e7681,false);

        // Scrubber
        let position=self.media.as_ref().map(|m|m.position_seconds()).unwrap_or(self.track.position_seconds);
        let duration=self.track.duration_seconds;
        let ratio=if duration>0.{(position/duration).clamp(0.,1.)}else{0.};

        let scrub_y=114.;
        p.text(&time_label(position),24.,scrub_y,30.,14.,10.,0xffffff,false);
        let track_x=58.;let track_w=128.;
        let scrub_hov=self.track.can_seek&&self.pointer.is_some_and(|(px,py)|px>=track_x&&px<=track_x+track_w&&py>=scrub_y&&py<=scrub_y+16.);
        p.fill(track_x,scrub_y+5.,track_w,4.,2.,if scrub_hov{0x52605e}else{0x3e4948});
        if ratio>0.{
            p.fill(track_x,scrub_y+5.,track_w*ratio as f32,4.,2.,0xffffff);
        }
        if scrub_hov{
            let thumb_x=(track_x+track_w*ratio as f32-4.5).clamp(track_x-2.,track_x+track_w-7.);
            p.fill(thumb_x,scrub_y+2.5,9.,9.,4.5,0xffffff);
        }
        if self.focused.as_ref()==Some(&Action::SeekPointer){
            p.stroke(track_x-2.,scrub_y+1.,track_w+4.,12.,4.,0xd5d5dc,1.);
        }
        if self.track.can_seek&&self.hit_enabled{
            self.hits.push(Hit{rect:render::rect(track_x,scrub_y,track_w,16.),action:Action::SeekPointer});
        }
        let remaining=(duration-position).max(0.);
        p.text(&format!("-{}",time_label(remaining)),190.,scrub_y,36.,14.,10.,0xffffff,false);

        // Transport Controls
        let trans_y=138.;
        self.transport_btn(p,"LYRICS",24.,trans_y,28.,28.,Action::ToggleLyrics,true,true);
        self.transport_btn(p,"⏮",64.,trans_y,28.,28.,Action::Previous,false,self.track.can_previous);
        let play_symbol=if self.track.playing{"⏸"}else{"▶"};
        self.transport_btn(p,play_symbol,104.,trans_y,28.,28.,Action::Play,false,self.track.can_toggle);
        self.transport_btn(p,"⏭",144.,trans_y,28.,28.,Action::Next,false,self.track.can_next);

        // Subtle vertical divider (x = 238)
        p.fill_alpha(238.,44.,1.,122.,0.,0xffffff,0.12);

        // Right Half: Synchronized Lyrics (x = 252..w-20)
        let lyrics_x=252.;
        let lyrics_w=w-272.;
        let no_lyrics_msg = crate::i18n::t("music.no_lyrics", &lang);
        let loading_lyrics_msg = crate::i18n::t("music.loading_lyrics", &lang);
        if self.lyrics.is_empty(){
            if self.jobs.contains_key("Musik"){
                p.text(loading_lyrics_msg,lyrics_x,70.,lyrics_w,50.,13.,0x8dc4dd,false);
            }else if !self.plain_lyrics.is_empty()&&self.plain_lyrics!="Lirik belum tersedia untuk lagu ini."&&self.plain_lyrics!="Lyrics not available for this song."{
                let plain_lines:Vec<&str>=self.plain_lyrics.lines().filter(|l|!l.trim().is_empty()).collect();
                if plain_lines.is_empty(){
                    p.text(&self.plain_lyrics,lyrics_x,70.,lyrics_w,50.,13.,0x72777b,false);
                }else{
                    let start=self.lyric_offset.saturating_sub(1).min(plain_lines.len().saturating_sub(1));
                    for (i,line) in plain_lines.iter().skip(start).take(4).enumerate(){
                        let ly=46.+i as f32*26.;
                        let col=if i==0{0xe2e8f0}else{0x8a9291};
                        p.text_ellipsis(line,lyrics_x,ly,lyrics_w,22.,12.5,col,i==0);
                    }
                    if self.lyric_offset>0{
                        self.button_h(p,"Reset",w-64.,144.,46.,20.,Action::LyricsFollow,false);
                    }
                }
            }else{
                p.text(no_lyrics_msg,lyrics_x,70.,lyrics_w,50.,13.,0x72777b,false);
            }
        }else{
            let curr=super::lyric_display_index(&self.lyrics,position,self.lyric_offset);
            if curr>0{
                let prev_line=&self.lyrics[curr-1];
                let hov=self.track.can_seek&&self.pointer.is_some_and(|(px,py)|px>=lyrics_x&&px<=lyrics_x+lyrics_w&&py>=44.&&py<=70.);
                p.text_ellipsis(&prev_line.text,lyrics_x,46.,lyrics_w,24.,13.,if hov{0xb0b5ba}else{0x72777b},false);
                if self.focused.as_ref()==Some(&Action::Lyric(curr-1)){
                    p.stroke(lyrics_x-2.,44.,lyrics_w+4.,24.,4.,0xd5d5dc,1.);
                }
                if self.track.can_seek&&self.hit_enabled{
                    self.hits.push(Hit{rect:render::rect(lyrics_x,44.,lyrics_w,26.),action:Action::Lyric(curr-1)});
                }
            }
            let active_line=&self.lyrics[curr];
            p.text_ellipsis(&active_line.text,lyrics_x,74.,lyrics_w,28.,15.,0xffffff,true);
            if self.focused.as_ref()==Some(&Action::Lyric(curr)){
                p.stroke(lyrics_x-2.,72.,lyrics_w+4.,28.,4.,0xd5d5dc,1.);
            }
            if self.track.can_seek&&self.hit_enabled{
                self.hits.push(Hit{rect:render::rect(lyrics_x,72.,lyrics_w,30.),action:Action::Lyric(curr)});
            }
            if curr+1<self.lyrics.len(){
                let next_line=&self.lyrics[curr+1];
                let hov=self.track.can_seek&&self.pointer.is_some_and(|(px,py)|px>=lyrics_x&&px<=lyrics_x+lyrics_w&&py>=102.&&py<=128.);
                p.text_ellipsis(&next_line.text,lyrics_x,104.,lyrics_w,24.,13.,if hov{0xb0b5ba}else{0x72777b},false);
                if self.focused.as_ref()==Some(&Action::Lyric(curr+1)){
                    p.stroke(lyrics_x-2.,102.,lyrics_w+4.,24.,4.,0xd5d5dc,1.);
                }
                if self.track.can_seek&&self.hit_enabled{
                    self.hits.push(Hit{rect:render::rect(lyrics_x,102.,lyrics_w,26.),action:Action::Lyric(curr+1)});
                }
            }
            if curr+2<self.lyrics.len(){
                let next2_line=&self.lyrics[curr+2];
                p.text_ellipsis(&next2_line.text,lyrics_x,130.,lyrics_w,20.,11.,0x555555,false);
            }
            if self.lyric_offset>0{
                let follow_lbl = crate::i18n::t("music.follow", &lang);
                self.button_h(p,follow_lbl,w-64.,144.,46.,20.,Action::LyricsFollow,true);
            }
        }
    }}


    fn calendar_page(&mut self,p:&mut Painter){unsafe{
        if self.width<320.{return}
        let lang = self.settings.language.clone();
        let month_name_str = crate::i18n::full_month_name(self.month, &lang);

        // LEFT COLUMN (Month Grid): x = 24..296
        p.text(&format!("{} {}",month_name_str,self.year),24.,44.,160.,20.,15.,0xffffff,true);
        self.button_icon(p,"‹",236.,42.,24.,24.,Action::Month(-1));
        self.button_icon(p,"›",266.,42.,24.,24.,Action::Month(1));

        let col_w=272./7.;
        for i in 0..7 {
            let name = crate::i18n::short_day_name(i, &lang);
            let color=if i==6{0xff7d7d}else{0x707077};
            p.text_centered(name,24.+i as f32*col_w,70.,col_w,16.,10.,color,1.0,false);
        }

        let first=NaiveDate::from_ymd_opt(self.year,self.month,1).unwrap_or(Local::now().date_naive());
        let shift=first.weekday().num_days_from_monday();
        let days_in_month=match self.month{
            1|3|5|7|8|10|12=>31,
            4|6|9|11=>30,
            2=>if self.year%4==0&&(self.year%100!=0||self.year%400==0){29}else{28},
            _=>30,
        };

        let today=Local::now().date_naive();
        for day in 1..=days_in_month{
            let Some(date)=NaiveDate::from_ymd_opt(self.year,self.month,day)else{continue};
            let index=shift+day-1;
            let col=index%7;
            let row=index/7;
            let x=24.+col as f32*col_w;
            let y=88.+row as f32*24.;

            let is_sunday=date.weekday().num_days_from_monday()==6;
            let holiday=self.holidays.iter().find(|h|h.date==date);
            let is_holiday=holiday.is_some_and(|h|!h.collective_leave);
            let is_leave=holiday.is_some_and(|h|h.collective_leave);
            let is_selected=self.day==day;
            let is_today=date==today;
            let day_act=Action::Day(day);
            let day_hov=self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+col_w&&py>=y-2.&&py<=y+22.);

            if is_selected{
                p.fill(x+4.,y-1.,col_w-8.,22.,6.,0xeaeaea);
                p.text_centered(&day.to_string(),x+4.,y-1.,col_w-8.,22.,12.,0x090909,1.0,true);
            }else{
                if day_hov{
                    p.fill_alpha(x+4.,y-1.,col_w-8.,22.,6.,0xffffff,0.08);
                }
                if is_today{
                    p.stroke(x+4.,y-1.,col_w-8.,22.,6.,0x555555,1.0);
                }
                let color=if is_holiday||is_sunday{
                    0xff7d7d
                }else if is_leave&&self.settings.collective_leave{
                    0xffa46b
                }else if day_hov{
                    0xffffff
                }else{
                    0xc6c6cc
                };
                p.text_centered(&day.to_string(),x+4.,y-1.,col_w-8.,22.,12.,color,1.0,is_today);
            }
            if self.focused.as_ref()==Some(&day_act){
                p.stroke(x+3.,y-2.,col_w-6.,24.,7.,0xd5d5dc,1.0);
            }

            // Dot indicators below date
            if is_holiday{
                p.fill(x+col_w*0.5-1.5,y+18.,3.,3.,1.5,0xff7d7d);
            }else if is_leave&&self.settings.collective_leave{
                p.fill(x+col_w*0.5-1.5,y+18.,3.,3.,1.5,0xffa46b);
            }

            if self.hit_enabled{
                self.hits.push(Hit{rect:render::rect(x,y-2.,col_w,24.),action:day_act});
            }
        }

        // Calendar legend at y = 232
        p.fill(24.,236.,4.,4.,2.,0xff7d7d);
        p.text(crate::i18n::t("calendar.national_holiday", &lang),32.,232.,80.,14.,10.,0x77777c,false);
        p.fill(124.,236.,4.,4.,2.,0xffa46b);
        p.text(crate::i18n::t("calendar.collective_leave", &lang),132.,232.,80.,14.,10.,0x77777c,false);

        // Footer at y = 254
        let footer_text = if self.year==2026 {
            crate::i18n::t("calendar.footer_2026", &lang)
        } else {
            crate::i18n::t("calendar.footer_other", &lang)
        };
        p.text(footer_text,24.,254.,184.,16.,9.,0x909096,false);
        let leave_label = if self.settings.collective_leave {
            crate::i18n::t("calendar.leave_toggle_on", &lang)
        } else {
            crate::i18n::t("calendar.leave_toggle_off", &lang)
        };
        let leave_hov=self.pointer.is_some_and(|(px,py)|px>=198.&&px<=292.&&py>=250.&&py<=270.);
        p.text(leave_label,200.,254.,90.,16.,9.,if leave_hov{0xefeff3}else{0x9c9ca3},false);
        if self.focused.as_ref()==Some(&Action::CalendarLeaveToggle){
            p.stroke(198.,250.,94.,20.,4.,0xd5d5dc,1.0);
        }
        if self.hit_enabled{
            self.hits.push(Hit{rect:render::rect(198.,250.,94.,20.),action:Action::CalendarLeaveToggle});
        }

        // Vertical Divider line between columns (x = 308)
        p.fill_alpha(308.,44.,1.,226.,0.,0xffffff,0.08);

        // RIGHT COLUMN (Date Detail & Upcoming Holiday): x = 324..516
        if let Some(date)=NaiveDate::from_ymd_opt(self.year,self.month,self.day){
            let day_idx=date.weekday().num_days_from_monday() as usize;
            p.text(crate::i18n::day_name_upper(day_idx, &lang),324.,44.,160.,14.,10.,0xffa46b,true);
            p.text(&self.day.to_string(),324.,56.,80.,46.,42.,0xffffff,true);
            p.text(&format!("{} {}",month_name_str,self.year),324.,104.,160.,16.,11.,0x9b9ba0,false);

            let holiday=self.holidays.iter().find(|h|h.date==date);
            let status_text=if self.year!=2026{
                crate::i18n::t("calendar.no_holiday_data", &lang).to_string()
            }else if let Some(h)=holiday{
                let prefix = if h.collective_leave {
                    crate::i18n::t("calendar.collective_leave", &lang)
                } else {
                    crate::i18n::t("calendar.national_holiday", &lang)
                };
                format!("{} · {}", prefix, h.name)
            }else if date==today{
                crate::i18n::t("calendar.today_regular", &lang).to_string()
            }else if date.weekday().num_days_from_monday()==6{
                crate::i18n::t("calendar.sunday_weekend", &lang).to_string()
            }else{
                crate::i18n::t("calendar.regular_day", &lang).to_string()
            };
            let status_color=if holiday.is_some_and(|h|!h.collective_leave){0xff7d7d}else if holiday.is_some(){0xffa46b}else{0x88888e};
            p.text_ellipsis(&status_text,324.,122.,188.,18.,11.,status_color,false);

            // Button "Hari ini"
            let today_hov=self.pointer.is_some_and(|(px,py)|px>=324.&&px<=388.&&py>=146.&&py<=170.);
            p.fill(324.,146.,64.,24.,6.,if today_hov{0x2b2b30}else{0x222222});
            p.stroke(324.,146.,64.,24.,6.,if today_hov{0x484852}else{0x333333},1.0);
            p.text_centered(crate::i18n::t("calendar.today_btn", &lang),324.,146.,64.,24.,10.,if today_hov{0xffffff}else{0xdddddd},1.0,false);
            if self.focused.as_ref()==Some(&Action::CalendarToday){
                p.stroke(322.,144.,68.,28.,8.,0xd5d5dc,1.0);
            }
            if self.hit_enabled{
                self.hits.push(Hit{rect:render::rect(324.,146.,64.,24.),action:Action::CalendarToday});
            }

            // Divider
            p.fill_alpha(324.,178.,188.,1.,0.,0xffffff,0.08);

            // Upcoming Holiday
            p.text(crate::i18n::t("calendar.next_holiday", &lang),324.,186.,188.,14.,8.,0x88888e,true);

            // Find next upcoming holiday strictly >= today
            let next_h=self.holidays.iter().filter(|h|h.date>=today&&!h.collective_leave).min_by_key(|h|h.date);
            if let Some(nh)=next_h{
                let card_y=202.;
                let jump_act=Action::JumpToDate(nh.date.year(),nh.date.month(),nh.date.day());
                let card_hov=self.pointer.is_some_and(|(px,py)|px>=324.&&px<=512.&&py>=card_y&&py<=card_y+42.);
                p.fill(324.,card_y,188.,42.,8.,if card_hov{0x221c1f}else{0x181517});
                p.stroke(324.,card_y,188.,42.,8.,if card_hov{0x3d2e31}else{0x292022},1.0);

                // Date badge
                p.fill(328.,card_y+3.,34.,36.,6.,0x2f171a);
                p.text_centered(&nh.date.day().to_string(),328.,card_y+4.,34.,18.,14.,0xff9a9a,1.0,true);
                let short_m = crate::i18n::month_name(nh.date.month(), &lang).to_uppercase();
                p.text_centered(&short_m,328.,card_y+20.,34.,14.,8.,0xff9a9a,0.9,true);

                // Title & Date description
                p.text_ellipsis(&nh.name,368.,card_y+6.,124.,16.,11.,0xffffff,true);
                let day_name_str = crate::i18n::day_name_capitalized(nh.date.weekday().num_days_from_monday() as usize, &lang);
                let holiday_month_str = crate::i18n::full_month_name(nh.date.month(), &lang);
                let date_desc=format!("{}, {} {}",day_name_str,nh.date.day(),holiday_month_str);
                p.text_ellipsis(&date_desc,368.,card_y+22.,124.,14.,10.,0x83838a,false);

                // Arrow icon
                p.text("↗",494.,card_y+12.,14.,16.,12.,if card_hov{0xffffff}else{0x888888},false);

                if self.focused.as_ref()==Some(&jump_act){
                    p.stroke(322.,card_y-2.,192.,46.,10.,0xd5d5dc,1.0);
                }

                // Clicking holiday card jumps directly to that exact holiday date!
                if self.hit_enabled{
                    self.hits.push(Hit{rect:render::rect(324.,card_y,188.,42.),action:jump_act});
                }
            }
        }
    }}

    fn clipboard_page(&mut self, p: &mut Painter) { unsafe {
        let lang = self.settings.language.clone();
        let w = self.width;
        let left_x = 22.;
        let left_w = 212.;

        let filter = self.clip_filter.trim().to_lowercase();
        let mut matching_indices: Vec<usize> = self.state.clipboard_history
            .iter()
            .enumerate()
            .filter(|(_, c)| filter.is_empty() || c.text.to_lowercase().contains(&filter))
            .map(|(idx, _)| idx)
            .collect();

        matching_indices.sort_by_key(|&idx| !self.state.clipboard_history[idx].pinned);
        let total_clips = matching_indices.len();

        if self.selected_clip >= total_clips && total_clips > 0 {
            self.selected_clip = total_clips - 1;
        }

        // ==========================================
        // LEFT PANE: Header & History List (x = 22..234, w = 212)
        // ==========================================
        let search_active = self.clip_search_open || !self.clip_filter.is_empty();

        if !search_active {
            let search_btn_hover = self.pointer.is_some_and(|(px, py)| px >= left_x && px <= left_x + 28. && py >= 41. && py <= 63.);
            if search_btn_hover {
                p.fill_alpha(left_x, 41., 28., 22., 6., 0xffffff, 0.10);
            }
            p.outline_icon("search", left_x + 6., 44., 15., if search_btn_hover { 0xffffff } else { 0x94a3b8 });
            if self.hit_enabled {
                self.hits.push(Hit { rect: render::rect(left_x, 41., 28., 22.), action: Action::ToggleClipSearch });
            }

            if !self.state.clipboard_history.is_empty() {
                self.button_h(p, crate::i18n::t("clipboard.clear", &lang), left_x + left_w - 70., 42., 70., 21., Action::ClearClips, false);
            }
        } else {
            p.outline_icon("search", left_x + 2., 44., 15., 0x38bdf8);

            let close_x = left_x + left_w - 22.;
            let close_hover = self.pointer.is_some_and(|(px, py)| px >= close_x && px <= close_x + 22. && py >= 41. && py <= 63.);
            if close_hover {
                p.fill_alpha(close_x, 41., 22., 22., 5., 0xffffff, 0.12);
            }
            p.text_centered("✕", close_x, 41., 22., 22., 11., if close_hover { 0xffffff } else { 0x94a3b8 }, 1.0, false);
            if self.hit_enabled {
                self.hits.push(Hit { rect: render::rect(close_x, 41., 22., 22.), action: Action::ToggleClipSearch });
            }

            p.fill(left_x, 63., left_w, 1.5, 0.5, 0x38bdf8);
        }

        if total_clips == 0 {
            let empty_msg = if !self.clip_filter.is_empty() {
                if lang == "en" { "No matching clips found." } else { "Tidak ada klip yang cocok." }
            } else {
                crate::i18n::t("clipboard.no_history", &lang)
            };
            p.text(empty_msg, left_x, 80., left_w, 20., 11., 0x6e7681, false);
            if self.clip_filter.is_empty() {
                p.text(crate::i18n::t("clipboard.hint", &lang), left_x, 104., left_w, 54., 10., 0x475569, false);
            }
        } else {
            let start = self.clip_offset.min(total_clips.saturating_sub(1));
            let visible_items: Vec<(usize, usize, crate::clipboard::ClipItem)> = matching_indices
                .iter()
                .enumerate()
                .skip(start)
                .take(4)
                .map(|(list_idx, &orig_idx)| (list_idx, orig_idx, self.state.clipboard_history[orig_idx].clone()))
                .collect();

            for (i, (list_idx, _orig_idx, item)) in visible_items.into_iter().enumerate() {
                let item_y = 68. + i as f32 * 36.;
                let is_selected = list_idx == self.selected_clip;
                let item_act = Action::SelectClip(list_idx);
                let item_hovered = self.pointer.is_some_and(|(px, py)| px >= left_x && px <= left_x + left_w && py >= item_y && py <= item_y + 32.);

                if is_selected {
                    p.fill_alpha(left_x, item_y, left_w, 32., 6., 0x0284c7, 0.28);
                    p.stroke(left_x, item_y, left_w, 32., 6., 0x38bdf8, 1.2);
                } else if item_hovered {
                    p.fill_alpha(left_x, item_y, left_w, 32., 6., 0xffffff, 0.08);
                    p.stroke(left_x, item_y, left_w, 32., 6., 0xffffff, 0.12);
                } else {
                    p.fill_alpha(left_x, item_y, left_w, 32., 6., 0xffffff, 0.03);
                    p.stroke(left_x, item_y, left_w, 32., 6., 0xffffff, 0.05);
                }
                if self.focused.as_ref() == Some(&item_act) {
                    p.stroke(left_x - 1., item_y - 1., left_w + 2., 34., 7., 0xd5d5dc, 1.0);
                }

                if item.pinned {
                    p.text("\u{1F4CC}", left_x + left_w - 20., item_y + 8., 16., 16., 10.0, 0xfacc15, false);
                }

                let first_line = item.text.lines().next().unwrap_or("").trim();
                let empty_txt = crate::i18n::t("clipboard.empty_text", &lang);
                let display_title = if first_line.is_empty() { empty_txt } else { first_line };
                let title_max_w = if item.pinned { left_w - 36. } else { left_w - 16. };
                p.text_ellipsis(display_title, left_x + 8., item_y + 3., title_max_w, 15., 11.0, if is_selected { 0xffffff } else { 0xe2e8f0 }, is_selected);

                let meta = crate::i18n::clip_meta(item.chars, item.lines, &lang);
                let full_meta = format!("{} \u{00B7} {}", item.time, meta);
                p.text(&full_meta, left_x + 8., item_y + 17., left_w - 16., 12., 9.0, if is_selected { 0x93c5fd } else { 0x64748b }, false);

                if self.hit_enabled {
                    self.hits.push(Hit {
                        rect: render::rect(left_x, item_y, left_w, 32.),
                        action: item_act,
                    });
                }
            }

            if total_clips > 4 {
                let page_info = format!("{}-{} / {}", start + 1, (start + 4).min(total_clips), total_clips);
                p.text(&page_info, left_x + 4., 214., 90., 16., 9.5, 0x64748b, false);
                self.button_h(p, "▲", left_x + 150., 212., 26., 18., Action::ClipPage(-1), false);
                self.button_h(p, "▼", left_x + 182., 212., 26., 18., Action::ClipPage(1), false);
            }
        }

        // ==========================================
        // SUBTLE VERTICAL DIVIDER (x = 244, matching lyric mode!)
        // ==========================================
        p.fill_alpha(244., 44., 1., 182., 0., 0xffffff, 0.10);

        // ==========================================
        // RIGHT PANE: Content Viewer & Action (x = 256..w-20)
        // ==========================================
        let right_x = 256.;
        let right_w = w - right_x - 20.;

        let selected_item = matching_indices.get(self.selected_clip).and_then(|&orig_idx| {
            self.state.clipboard_history.get(orig_idx).cloned().map(|it| (orig_idx, it))
        });

        if total_clips == 0 {
            p.text_centered("📋", right_x, 82., right_w, 28., 24., 0x38bdf8, 0.8, false);
            p.text_centered(crate::i18n::t("clipboard.empty_title", &lang), right_x, 114., right_w, 20., 13., 0xffffff, 1.0, true);
            p.text_centered(crate::i18n::t("clipboard.empty_desc1", &lang), right_x, 138., right_w, 16., 10.5, 0x8a9291, 1.0, false);
            p.text_centered(crate::i18n::t("clipboard.empty_desc2", &lang), right_x, 154., right_w, 16., 10.5, 0x8a9291, 1.0, false);
        } else if let Some((orig_idx, item)) = selected_item {
            let clip_word = crate::i18n::t("clipboard.clip", &lang);
            let title = format!("{} #{} \u{00B7} {}", clip_word, self.selected_clip + 1, item.time);
            p.text(&title, right_x, 46., 120., 18., 11.5, 0x38bdf8, true);

            let pin_label = if item.pinned { crate::i18n::t("clipboard.unpin", &lang) } else { crate::i18n::t("clipboard.pin", &lang) };
            let pin_w = 68.;
            let pin_x = w - 20. - 98. - 8. - pin_w;
            self.button_h(p, pin_label, pin_x, 43., pin_w, 22., Action::TogglePinClip(item.id), item.pinned);

            let btn_w = 98.;
            let btn_x = w - 20. - btn_w;
            self.button_h(p, crate::i18n::t("clipboard.recopy", &lang), btn_x, 43., btn_w, 22., Action::CopyClip(orig_idx), true);

            let box_y = 69.;
            let box_h = 138.;
            p.fill_alpha(right_x, box_y, right_w, box_h, 8., 0x0f172a, 0.75);
            p.stroke(right_x, box_y, right_w, box_h, 8., 0x1e293b, 1.0);

            let all_lines: Vec<&str> = item.text.lines().collect();
            let max_lines = 6;
            let display_lines = all_lines.len().min(max_lines);

            for (li, line) in all_lines.iter().take(display_lines).enumerate() {
                let line_y = box_y + 8. + li as f32 * 18.;
                p.text_ellipsis(line, right_x + 10., line_y, right_w - 20., 16., 10.5, 0xf1f5f9, false);
            }

            if all_lines.len() > max_lines {
                let more_y = box_y + 8. + max_lines as f32 * 18.;
                let more_text = crate::i18n::more_lines(all_lines.len() - max_lines, &lang);
                p.text(&more_text, right_x + 10., more_y, right_w - 20., 14., 9.5, 0x64748b, false);
            }

            let footer_y = 212.;
            let count_meta = crate::i18n::clip_meta(item.chars, item.lines, &lang);
            p.text(&count_meta, right_x + 4., footer_y + 2., 140., 14., 9.5, 0x64748b, false);
            self.button_h(p, crate::i18n::t("clipboard.delete", &lang), w - 20. - 68., footer_y, 68., 20., Action::DeleteClip(item.id), false);
        }
    }}

    fn notes_page(&mut self, p: &mut Painter) { unsafe {
        let lang = self.settings.language.clone();
        let w = self.width;
        let left_x = 22.;
        let left_w = 212.;

        // ==========================================
        // LEFT PANE: Sticky Notes List
        // ==========================================
        p.text(crate::i18n::t("notes.header", &lang), left_x, 46., 120., 18., 11.5, 0xfef08a, true);
        self.button_h(p, crate::i18n::t("notes.new_btn", &lang), left_x + 140., 43., 72., 20., Action::NoteAdd, true);

        let total_notes = self.state.sticky_notes.len();
        if total_notes == 0 {
            p.text(crate::i18n::t("notes.no_notes", &lang), left_x + 6., 86., left_w - 12., 18., 11.0, 0x94a3b8, false);
            p.text(crate::i18n::t("notes.empty_hint", &lang), left_x + 6., 108., left_w - 12., 36., 9.5, 0x64748b, false);
        } else {
            let max_visible = 5;
            if self.state.selected_note >= total_notes {
                self.state.selected_note = total_notes.saturating_sub(1);
            }
            let start = if total_notes <= max_visible {
                0
            } else {
                self.state.selected_note.saturating_sub(max_visible - 1).min(total_notes - max_visible)
            };
            let end = (start + max_visible).min(total_notes);

            for (slot, idx) in (start..end).enumerate() {
                let note = &self.state.sticky_notes[idx];
                let item_y = 68. + slot as f32 * 35.;
                let is_selected = idx == self.state.selected_note;
                let item_act = Action::NoteSelect(idx);
                let item_hovered = self.pointer.is_some_and(|(px, py)| px >= left_x && px <= left_x + left_w && py >= item_y && py <= item_y + 32.);

                if is_selected {
                    p.fill_alpha(left_x, item_y, left_w, 32., 6., 0xfef08a, 0.16);
                    p.stroke(left_x, item_y, left_w, 32., 6., 0xfef08a, 1.2);
                } else if item_hovered {
                    p.fill_alpha(left_x, item_y, left_w, 32., 6., 0xffffff, 0.08);
                    p.stroke(left_x, item_y, left_w, 32., 6., 0xffffff, 0.12);
                } else {
                    p.fill_alpha(left_x, item_y, left_w, 32., 6., 0xffffff, 0.03);
                    p.stroke(left_x, item_y, left_w, 32., 6., 0xffffff, 0.05);
                }

                if self.focused.as_ref() == Some(&item_act) {
                    p.stroke(left_x - 1., item_y - 1., left_w + 2., 34., 7., 0xd5d5dc, 1.0);
                }

                let (note_col, _) = crate::sticky::NOTE_COLORS.get(note.color_idx as usize).copied().unwrap_or((0xfef08a, "Kuning"));
                p.fill(left_x + 6., item_y + 8., 4., 16., 2., note_col);

                let default_note_title = crate::i18n::t("notes.default_title", &lang);
                let title = if note.title.trim().is_empty() || (note.title == "Catatan Baru" && lang == "en") {
                    default_note_title
                } else {
                    &note.title
                };
                p.text_ellipsis(title, left_x + 16., item_y + 3., left_w - 24., 15., 11.0, if is_selected { 0xffffff } else { 0xe2e8f0 }, is_selected);

                let empty_snippet = crate::i18n::t("notes.empty_snippet", &lang);
                let snippet = if note.content.trim().is_empty() { empty_snippet } else { note.content.lines().next().unwrap_or("").trim() };
                p.text_ellipsis(snippet, left_x + 16., item_y + 17., left_w - 24., 12., 9.0, if is_selected { 0xfef08a } else { 0x64748b }, false);

                if self.hit_enabled {
                    self.hits.push(Hit {
                        rect: render::rect(left_x, item_y, left_w, 32.),
                        action: item_act,
                    });
                }
            }

            let info = crate::i18n::notes_count(total_notes, &lang);
            p.text(&info, left_x + 4., 248., 120., 16., 9.5, 0x64748b, false);
        }

        // ==========================================
        // SUBTLE VERTICAL DIVIDER (x = 244)
        // ==========================================
        p.fill_alpha(244., 44., 1., 220., 0., 0xffffff, 0.10);

        // ==========================================
        // RIGHT PANE: Editor & Controls (x = 256..w-20)
        // ==========================================
        let right_x = 256.;
        let right_w = w - right_x - 20.;

        if total_notes == 0 {
            p.text_centered("✎", right_x, 82., right_w, 28., 24., 0xfef08a, 0.8, false);
            p.text_centered("Quick Sticky Notes", right_x, 114., right_w, 20., 13., 0xffffff, 1.0, true);
            p.text_centered(crate::i18n::t("notes.empty_desc1", &lang), right_x, 138., right_w, 16., 10.5, 0x8a9291, 1.0, false);
            p.text_centered(crate::i18n::t("notes.empty_desc2", &lang), right_x, 154., right_w, 16., 10.5, 0x8a9291, 1.0, false);
        } else if let Some(note) = self.state.sticky_notes.get(self.state.selected_note).cloned() {
            self.button_h(p, ".md", 402., 44., 34., 22., Action::NoteExport(0), false);
            self.button_h(p, ".txt", 440., 44., 34., 22., Action::NoteExport(1), false);
            self.button_h(p, "⧉ Pop", 478., 44., 42., 22., Action::NotePopout, true);

            let (bg_col, _) = crate::sticky::NOTE_COLORS.get(note.color_idx as usize).copied().unwrap_or((0xfef08a, "Kuning"));
            p.fill_alpha(right_x - 2., 72., right_w + 4., 144., 6., bg_col, 0.12);
            p.stroke(right_x - 2., 72., right_w + 4., 144., 6., bg_col, 0.6);

            for (ci, &(c_val, _)) in crate::sticky::NOTE_COLORS.iter().enumerate() {
                let cx = right_x + ci as f32 * 20.;
                let cy = 224.;
                let is_color_sel = note.color_idx == ci as u8;
                p.fill(cx, cy, 14., 14., 3., c_val);
                if is_color_sel {
                    p.stroke(cx - 2., cy - 2., 18., 18., 4., 0xffffff, 1.5);
                }
                if self.hit_enabled {
                    self.hits.push(Hit {
                        rect: render::rect(cx - 2., cy - 2., 18., 18.),
                        action: Action::NoteColor(ci as u8),
                    });
                }
            }

            self.button_h(p, crate::i18n::t("notes.copy", &lang), 368., 221., 64., 20., Action::NoteExport(2), false);
            self.button_h(p, crate::i18n::t("notes.delete", &lang), 436., 221., 64., 20., Action::NoteDelete(note.id), false);

            let updated_lbl = crate::i18n::t("notes.updated", &lang);
            let update_lbl = format!("{}: {}", updated_lbl, note.updated_at);
            p.text(&update_lbl, right_x, 248., 240., 14., 9.0, 0x64748b, false);
        }
    }}

    pub(super) fn action(&mut self,action:Action){
        let transient=matches!(&action,Action::SettingsCategory(..)|Action::SettingsMenu(..)|Action::Settings|Action::Back|Action::Tab(..)|Action::ReminderMenu(..)|Action::ReminderSelect(..)|Action::ReminderStep(..)|Action::WeatherRefresh|Action::WeatherToggleMenu|Action::WeatherToggleCustom);
        match action{
            Action::TogglePin=>{
                self.pinned=!self.pinned;
                let notice_text = if self.pinned {
                    crate::i18n::t("notice.pinned", &self.settings.language)
                } else {
                    crate::i18n::t("notice.unpinned", &self.settings.language)
                };
                self.notice(notice_text);
                self.invalidate();
            },
            Action::ToStage0=>{
                self.hover_blocked_until=Instant::now()+Duration::from_millis(600);
                self.set_hover_stage(0);
            },
            Action::ToStage1=>{
                self.hover_blocked_until=Instant::now()+Duration::from_millis(600);
                self.set_hover_stage(super::collapse_target_stage(self.settings.minimal));
            },
            Action::CalendarToday=>{
                let now=Local::now().date_naive();
                self.year=now.year();
                self.month=now.month();
                self.day=now.day();
                self.load_holidays();
            },
            Action::JumpToDate(y,m,d)=>{
                self.year=y;
                self.month=m;
                self.day=d;
                self.load_holidays();
            },
            Action::CalendarLeaveToggle=>{
                self.settings.collective_leave=!self.settings.collective_leave;
            },
            Action::Tab(tab)=>{
                self.stage1_media_until=None;
                self.reminder_menu=None;
                self.mod_view=None;
                if self.tab == tab && self.hover_stage >= 2 {
                    self.hover_blocked_until=Instant::now()+Duration::from_millis(600);
                    self.set_hover_stage(super::collapse_target_stage(self.settings.minimal));
                    return;
                }
                if self.tab!=tab{
                    let order = self.settings.active_tab_order();
                    let old_idx=order.iter().position(|m|m==&self.tab).unwrap_or(0);
                    let new_idx=order.iter().position(|m|m==&tab).unwrap_or(0);
                    let dir=if new_idx>=old_idx{1i8}else{-1i8};
                    self.lyric_mode=false;
                    self.tab_fade=Some((self.tab.clone(),Instant::now(),dir));
                    self.tab=tab;
                    if self.tab == "Cuaca" && self.weather.is_none() {
                        self.refresh_weather();
                    }
                    if self.hover_stage == 1 && self.should_revert_stage1_to_calendar() {
                        self.trigger_stage1_temporary(5);
                    }
                    self.settings_open=false;
                    let from=self.geometry();
                    self.morph=Some(ui::Morph::new(from,self.target_geometry()));
                    self.update_window_region();
                    self.start_morph_timer();
                    self.make_inputs();
                    self.refresh_mod_output();
                    self.invalidate();
                    unsafe{SetTimer(Some(self.hwnd),10,16,None);}
                }
            },
            Action::MoveTab(tab, delta)=>{
                self.settings.move_tab(&tab, delta);
                self.save();
                self.invalidate();
            },
            Action::SettingsCategory(cat)=>{self.settings_cat=cat.min(6);self.settings_popup=None;self.focused=Some(Action::SettingsCategory(self.settings_cat));if self.settings_cat==5{self.invalidate_mods();}},
            Action::InstallMod(id)=>{
                let root=self.store.root.clone();
                match crate::mods::install_mod(&root,&mut self.settings,&id){
                    Ok(())=>{self.invalidate_mods();self.save();self.notice(crate::i18n::t("notice.mod_installed", &self.settings.language));}
                    Err(e)=>self.notice(format!("{} {e}",crate::i18n::t("error.mod_install", &self.settings.language))),
                }
                self.invalidate();
            },
            Action::UninstallMod(id)=>{
                let root=self.store.root.clone();
                match crate::mods::uninstall_mod(&root,&mut self.settings,&id){
                    Ok(()) => {
                        self.invalidate_mods();
                        if !self.settings.enabled(&self.tab){
                            self.tab=self.settings.active_tab_order().first().cloned().unwrap_or_else(||"Musik".into());
                            self.make_inputs();
                        }
                        self.save();
                        self.notice(crate::i18n::t("notice.mod_uninstalled", &self.settings.language));
                    }
                    Err(e)=>self.notice(format!("{} {e}",crate::i18n::t("error.mod_uninstall", &self.settings.language))),
                }
                self.invalidate();
            },
            Action::DeleteCustomMod(id)=>{
                let root=self.store.root.clone();
                match crate::mods::delete_custom_mod(&root,&mut self.settings,&id){
                    Ok(()) => {
                        self.invalidate_mods();
                        if !self.settings.enabled(&self.tab){
                            self.tab=self.settings.active_tab_order().first().cloned().unwrap_or_else(||"Musik".into());
                            self.make_inputs();
                        }
                        self.save();
                        self.notice(crate::i18n::t("notice.mod_deleted", &self.settings.language));
                    }
                    Err(e)=>self.notice(format!("{} {e}",crate::i18n::t("error.mod_delete", &self.settings.language))),
                }
                self.invalidate();
            },
            Action::PickAndInstallMod=>{
                if let Some(path) = crate::system::pick_mod_file() {
                    let root = self.store.root.clone();
                    match crate::mods::install_mod_package(&root, &mut self.settings, &path) {
                        Ok(manifest)=>{
                            self.invalidate_mods();
                            self.mods_selected=Some(manifest.id.clone());
                            self.save();
                            self.notice(format!("{} {}",crate::i18n::t("notice.mod_import_ok", &self.settings.language), manifest.name));
                        }
                        Err(e)=>self.notice(format!("{} {e}",crate::i18n::t("error.mod_install", &self.settings.language))),
                    }
                    self.invalidate();
                }
            },
            Action::SelectMod(id)=>{
                self.mods_selected=Some(id);
                self.invalidate();
            },
            Action::OpenMod(id)=>{
                if !self.mods().iter().any(|m| m.id==id){
                    self.notice(crate::i18n::t("mods.missing", &self.settings.language));
                    return;
                }
                self.mod_view=Some(id);
                self.settings_open=false;
                self.settings_popup=None;
                self.reminder_menu=None;
                self.focused=None;
                self.tab_fade=None;
                let from=self.geometry();
                self.morph=Some(ui::Morph::new(from,self.target_geometry()));
                self.update_window_region();
                self.start_morph_timer();
                self.make_inputs();
                self.refresh_mod_output();
                self.invalidate();
            },
            Action::CloseModView=>{
                self.mod_view=None;
                self.settings_open=true;
                self.settings_cat=5;
                self.settings_popup=None;
                self.focused=None;
                let from=self.geometry();
                self.morph=Some(ui::Morph::new(from,self.target_geometry()));
                self.update_window_region();
                self.start_morph_timer();
                self.make_inputs();
                self.invalidate();
            },
            Action::OpenModsFolder=>{
                let mods_dir = self.store.root.join("mods");
                if let Err(e) = crate::system::open_folder_in_explorer(&mods_dir) {
                    self.notice(format!("{} {e}",crate::i18n::t("error.open_folder", &self.settings.language)));
                }
                self.invalidate_mods();
            },
            Action::RefreshMods=>{self.invalidate_mods();self.invalidate();},
            Action::ToggleModTab(tab)=>{
                let on=!self.settings.enabled(&tab);
                self.settings.set_mod_tab(&tab,on);
                if self.tab==tab&&!self.settings.enabled(&tab){
                    self.tab=self.settings.active_tab_order().first().cloned().unwrap_or_else(||"Musik".into());
                    self.make_inputs();
                }
                self.save();
                self.invalidate();
            },
            Action::ToggleModNetwork(id)=>{
                let on=!self.settings.network_mods.iter().any(|m|m==&id);
                self.settings.set_mod_network(&id,on);
                self.save();
                if self.active_mod_id().as_deref()==Some(id.as_str()){
                    self.mod_output=None;
                    self.refresh_mod_output();
                }
                self.invalidate();
            },
            Action::ModButtonClick(cmd)=>{
                if let Some(url) = cmd.strip_prefix("url:") {
                    crate::system::open_browser(url);
                } else if let Some(msg) = cmd.strip_prefix("toast:") {
                    self.notice(msg);
                } else if let Some(txt) = cmd.strip_prefix("copy:") {
                    crate::clipboard::set_clipboard_text(txt);
                    self.notice(crate::i18n::t("notice.clip_copied", &self.settings.language));
                } else {
                    self.last_mod_button_clicked = Some(cmd);
                    self.refresh_mod_output();
                }
                self.invalidate();
            },
            Action::SettingsMenu(key,x,y,w)=>self.open_settings_menu(key,x,y,w),
            Action::SettingsValue(key,value)=>{self.apply_settings_value(key,value);self.settings_popup=None;},
            Action::SettingsOpenTab(tab,index)=>{if !self.settings.enabled(&tab){return}let from=self.geometry();self.mod_view=None;self.settings_open=false;self.settings_popup=None;self.reminder_menu=None;self.tab=tab;self.tab_fade=None;if self.tab=="Reminder"{self.select_reminder(index);}self.morph=Some(ui::Morph::new(from,self.target_geometry()));self.start_morph_timer();self.make_inputs();self.update_window_region();},
            Action::SetDefaultTab(tab)=>{if self.settings.enabled(&tab){self.settings.default_tab=tab;}},
            Action::Settings=>{self.settings_popup=None;self.reminder_menu=None;self.focused=None;self.mod_view=None;self.settings_open=true;self.set_expanded(true);let from=self.geometry();self.morph=Some(ui::Morph::new(from,self.target_geometry()));self.update_window_region();self.start_morph_timer();self.make_inputs();self.invalidate();},
            Action::Back=>{self.settings_popup=None;self.focused=None;self.mod_view=None;self.settings_open=false;let from=self.geometry();self.morph=Some(ui::Morph::new(from,self.target_geometry()));self.update_window_region();self.start_morph_timer();self.make_inputs();self.invalidate();},
            Action::Close=>self.set_expanded(false),Action::Exit=>{self.flush_note_edits();unsafe{let _=DestroyWindow(self.hwnd);}},
            Action::Bottom(value)=>{self.settings.bottom=value;self.layout();self.save();},
            Action::Minimal=>{self.settings.minimal=!self.settings.minimal;if !self.expanded{self.set_hover_stage(super::collapse_target_stage(self.settings.minimal));}self.sync_visualizer();self.save();},
            Action::Gap(delta)=>{self.settings.edge_gap=(self.settings.edge_gap+delta).clamp(0,160);self.layout();self.save();},Action::Scale(delta)=>{self.settings.scale=(self.settings.scale+delta).clamp(0.8,1.35);self.layout();self.make_inputs();self.save();},Action::Opacity(delta)=>{self.settings.opacity=(self.settings.opacity as i32+delta).clamp(MIN_OPACITY as i32,255)as u8;self.save();self.invalidate();},
            Action::ToggleModule(module)=>{let enabled=!self.settings.enabled(&module);if !self.settings.set_module_enabled(&module,enabled){self.notice(crate::i18n::t("notice.at_least_one_tab", &self.settings.language));return}if module=="Musik"{if enabled{self.start_media();}else{if let Some(mut m)=self.media.take(){m.stop();}self.track=MediaSnapshot::default();self.lyrics.clear();self.sync_visualizer();}}if module=="Reminder"&&!enabled{self.focus.reset();self.stretch_deadline=None;}if module=="Sehat"&&!enabled{self.breathing=None;}if !self.settings.enabled(&self.tab){self.tab=self.settings.active_tab_order().first().cloned().unwrap_or_default();self.make_inputs();}self.save();self.invalidate();},
            Action::Toggle(key)=>match key{
                "auto_break"=>self.settings.auto_break=!self.settings.auto_break,
                "startup"=>{
                    let new_val=!self.settings.run_at_startup;
                    if let Err(e)=crate::system::set_run_at_startup(new_val){
                        self.notice(&format!("Error: {}",e));
                    }else{
                        self.settings.run_at_startup=new_val;
                        let msg=if new_val{
                            if self.settings.language=="en"{"Run at Windows startup enabled."}else{"Jalankan saat startup Windows diaktifkan."}
                        }else{
                            if self.settings.language=="en"{"Run at Windows startup disabled."}else{"Jalankan saat startup Windows dinonaktifkan."}
                        };
                        self.notice(msg);
                    }
                },
                "focus_enabled"=>{self.settings.focus_enabled=!self.settings.focus_enabled;if !self.settings.focus_enabled{self.focus.pause();}},
                "stretch_after_focus"=>self.settings.stretch_after_focus=!self.settings.stretch_after_focus,
                "prayer_sound"=>self.settings.prayer_sound=!self.settings.prayer_sound,
                "prayer_enabled"=>{self.settings.prayer_enabled=!self.settings.prayer_enabled;self.refresh_prayers(false);},
                "message_preview"=>self.settings.message_preview=!self.settings.message_preview,
                "software_renderer"=>{
                    self.settings.software_renderer = !self.settings.software_renderer;
                    self.save();
                    let msg = if self.settings.software_renderer {
                        if self.settings.language == "en" { "Low RAM Mode enabled. Please restart app to apply." } else { "Mode Hemat RAM aktif. Muat ulang aplikasi untuk menerapkan." }
                    } else {
                        if self.settings.language == "en" { "Hardware rendering enabled. Please restart app to apply." } else { "Akselerasi GPU aktif. Muat ulang aplikasi untuk menerapkan." }
                    };
                    self.notice(msg);
                },
                "fullscreen"=>{self.settings.hide_fullscreen=!self.settings.hide_fullscreen;self.save();},
                "glow"=>{self.settings.glow=!self.settings.glow;self.save();},
                "visualizer"=>{self.settings.visualizer=!self.settings.visualizer;self.sync_visualizer();self.save();},
                "romanize_lyrics"=>{
                    self.settings.romanize_lyrics=!self.settings.romanize_lyrics;
                    self.lyrics_key.clear();
                    if !self.track.title.is_empty(){
                        let key=serde_json::to_string(&(&self.track.title,&self.track.artist,&self.track.album,self.track.duration_seconds.round()as u64)).unwrap_or_default();
                        self.lyrics_key=key.clone();
                        let api=self.online.clone();
                        let track=self.track.clone();
                        self.job("Musik",move||Event::Lyrics(key,api.lyrics(&track.title,&track.artist,&track.album,track.duration_seconds)));
                    }
                    self.save();
                },
                "water_reminder"=>{self.settings.water_reminder=!self.settings.water_reminder;self.last_water=Instant::now();self.save();},
                "move_reminder"=>{self.settings.move_reminder=!self.settings.move_reminder;self.last_move=Instant::now();self.save();},
                "lyrics_below"=>self.settings.lyrics_below=!self.settings.lyrics_below,
                "save_schedule"=>{let schedule=self.input(100);let times:Vec<String>=schedule.split(',').map(str::trim).filter(|s|!s.is_empty()).map(str::to_owned).collect();if times.iter().any(|t|chrono::NaiveTime::parse_from_str(t,"%H:%M").is_err()){self.notice(crate::i18n::t("notice.schedule_invalid", &self.settings.language));return}self.settings.focus_schedule=times;let tab=self.input(101);if MODULES.contains(&tab.trim()){self.settings.default_tab=tab.trim().into();}self.notice(crate::i18n::t("notice.schedule_saved", &self.settings.language));},
                _=>{}
            },
            Action::SeekPointer=>{},Action::LyricsFollow=>self.lyric_offset=0,
            Action::Play=>self.media_command(|m|m.toggle_playback()),Action::Next=>self.media_command(|m|m.next()),Action::Previous=>self.media_command(|m|m.previous()),Action::Seek(seconds)=>{if self.track.can_seek{self.media_command(|m|m.seek(seconds));}},Action::Lyric(index)=>{if self.track.can_seek{if let Some(line)=self.lyrics.get(index){let at=line.at;self.media_command(|m|m.seek(at));}}},
            Action::ReminderSelect(i)=>self.select_reminder(i),
            Action::ReminderStep(d)=>self.select_reminder((self.reminder_index as i32+d).clamp(0,3)as usize),
            Action::StretchLength(d)=>self.settings.stretch_seconds=(self.settings.stretch_seconds as i32+d).clamp(15,300)as u32,
            Action::StretchDone=>{self.stretch_deadline=None;self.last_move=Instant::now();self.notice(crate::i18n::t("notice.stretch_done", &self.settings.language));},
            Action::StretchStart=>{if self.stretch_deadline.is_some(){self.stretch_deadline=None;self.last_move=Instant::now();}else{self.stretch_deadline=Some(Instant::now()+Duration::from_secs(self.settings.stretch_seconds as u64));}},
            Action::PrayerRefresh=>self.refresh_prayers(true),
            Action::ReminderMenu(key)=>{self.open_reminder_menu(key);},
            Action::ReminderValue(key,value)=>{self.apply_reminder_value(key,value);self.reminder_menu=None;},
            Action::PrayerLead(d)=>self.settings.prayer_lead=(self.settings.prayer_lead as i32+d).clamp(0,30)as u32,

            Action::FocusStart=>self.focus.start(self.settings.focus_minutes,false),Action::FocusBreak=>self.focus.start(self.settings.break_minutes,true),Action::FocusPause=>{if self.focus.running(){self.focus.pause();}else{self.focus.resume();}},Action::FocusReset=>self.focus.reset(),Action::FocusLength(delta)=>{self.settings.focus_minutes=(self.settings.focus_minutes as i32+delta).clamp(1,180)as u32;self.save();},Action::BreakLength(delta)=>{self.settings.break_minutes=(self.settings.break_minutes as i32+delta).clamp(1,60)as u32;self.save();},
            Action::AddTodo=>{let text=self.input(10).trim().to_owned();if text.is_empty(){return}let due_text=self.input(11);let due=if due_text.trim().is_empty(){None}else{use chrono::TimeZone;match chrono::NaiveDateTime::parse_from_str(due_text.trim(),"%Y-%m-%d %H:%M").ok().and_then(|d|Local.from_local_datetime(&d).single()){Some(time)=>Some(time),None=>{self.notice(crate::i18n::t("notice.todo_due_invalid", &self.settings.language));return}}};self.state.todos.push(Todo{id:Local::now().timestamp_millis()as u64,text,done:false,due,reminded:false});self.clear_input(10);self.clear_input(11);},Action::Todo(id)=>{if let Some(t)=self.state.todos.iter_mut().find(|t|t.id==id){t.done=!t.done;}},Action::DeleteTodo(id)=>self.state.todos.retain(|t|t.id!=id),Action::TodoPage(delta)=>self.todo_offset=(self.todo_offset as i32+delta).clamp(0,self.state.todos.len().saturating_sub(1)as i32)as usize,Action::SaveNote=>{self.state.notes=self.input(12);self.notice(crate::i18n::t("notice.note_saved", &self.settings.language));},
            Action::Water=>{self.state.water.push(Drink{date:Local::now().date_naive(),ml:self.settings.water_serving});self.last_water=Instant::now();},Action::UndoWater=>{if let Some(index)=self.state.water.iter().rposition(|d|d.date==Local::now().date_naive()){self.state.water.remove(index);}},Action::WaterTarget(delta)=>self.settings.water_target=(self.settings.water_target as i32+delta).clamp(250,10000)as u32,Action::WaterServing(delta)=>self.settings.water_serving=(self.settings.water_serving as i32+delta).clamp(50,2000)as u32,
            Action::WaterInterval(delta)=>{self.settings.water_interval=(self.settings.water_interval as i32+delta).clamp(5,180)as u32;self.save();},
            Action::MoveInterval(delta)=>{self.settings.move_interval=(self.settings.move_interval as i32+delta).clamp(5,180)as u32;self.save();},
            Action::ConfirmWater=>{
                let serving=self.settings.water_serving;
                self.state.water.push(Drink{date:Local::now().date_naive(),ml:serving});
                self.last_water=Instant::now();
                self.save();
                if let Some(toast)=&mut self.toast{
                    toast.confirmed=true;
                    toast.confirmed_at=Some(Instant::now());
                    toast.message = if self.settings.language == "en" {
                        format!("Awesome! +{}ml logged 🎉", serving)
                    } else {
                        format!("Hebat! +{}ml tercatat 🎉", serving)
                    };
                }
                unsafe{
                    let _=MessageBeep(0x00000040);
                    SetTimer(Some(self.hwnd),3,2200,None);
                }
                let from=self.geometry();
                self.morph=Some(ui::Morph::new(from,self.target_geometry()));
                self.start_morph_timer();
                self.invalidate();
            },
            Action::SelectClip(idx)=>{self.selected_clip=idx;self.invalidate();},
            Action::CopyClip(idx)=>{
                if let Some(item)=self.state.clipboard_history.get(idx).cloned(){
                    if crate::clipboard::set_clipboard_text(&item.text){
                        self.last_clipboard_seq=crate::clipboard::get_sequence_number();
                        self.state.clipboard_history.retain(|c|c.id!=item.id);
                        self.state.clipboard_history.insert(0,item);
                        self.selected_clip=0;
                        self.save();
                        self.notice(crate::i18n::t("notice.clip_copied", &self.settings.language));
                    }else{
                        self.notice(crate::i18n::t("notice.clip_failed", &self.settings.language));
                    }
                }
            },
            Action::DeleteClip(id)=>{
                self.state.clipboard_history.retain(|c|c.id!=id);
                if self.selected_clip>=self.state.clipboard_history.len(){
                    self.selected_clip=self.state.clipboard_history.len().saturating_sub(1);
                }
                self.save();
                self.invalidate();
            },
            Action::ClearClipFilter=>{
                self.clip_filter.clear();
                self.clear_input(60);
                self.clip_offset=0;
                self.clip_search_open=false;
                self.make_inputs();
                self.invalidate();
            },
            Action::ToggleClipSearch=>{
                self.clip_search_open = !self.clip_search_open;
                if !self.clip_search_open {
                    self.clip_filter.clear();
                    self.clear_input(60);
                    self.clip_offset = 0;
                }
                self.make_inputs();
                if self.clip_search_open {
                    if let Some(&edit_hwnd) = self.inputs.get(&60) {
                        unsafe {
                            let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(edit_hwnd));
                        }
                    }
                }
                self.invalidate();
            },
            Action::TogglePinClip(id)=>{
                if let Some(item) = self.state.clipboard_history.iter_mut().find(|it| it.id == id) {
                    item.pinned = !item.pinned;
                    self.save();
                    self.invalidate();
                }
            },
            Action::ClearClips=>{
                self.state.clipboard_history.clear();
                self.selected_clip=0;
                self.clip_offset=0;
                self.save();
                self.notice(crate::i18n::t("notice.clips_cleared", &self.settings.language));
                self.invalidate();
            },
            Action::ClipPage(delta)=>{
                let max_offset=self.state.clipboard_history.len().saturating_sub(4);
                self.clip_offset=(self.clip_offset as i32+delta).clamp(0,max_offset as i32)as usize;
                self.invalidate();
            },
            Action::Breath=>self.breathing=if self.breathing.is_some(){None}else{Some(Instant::now())},
            Action::Month(delta)=>{let index=self.year*12+self.month as i32-1+delta;self.year=index.div_euclid(12);self.month=(index.rem_euclid(12)+1)as u32;self.day=1;self.load_holidays();},Action::Day(day)=>self.day=day,
            Action::CalendarSave=>{let feed=self.input(20);match crate::secrets::protect(feed.trim()){Ok(secret)=>{self.settings.calendar_feed=secret;self.clear_input(20);self.refresh_agenda();},Err(e)=>self.notice(e)}},Action::CalendarRefresh=>self.refresh_agenda(),
            Action::WeatherSave=>{
                let city = self.input(30).trim().to_string();
                match(self.input(31).trim().parse::<f64>(),self.input(32).trim().parse::<f64>()){
                    (Ok(lat),Ok(lon))if lat.is_finite()&&lon.is_finite()&&(-90.0..=90.0).contains(&lat)&&(-180.0..=180.0).contains(&lon)=>{
                        self.reset_weather_location();
                        self.settings.latitude=lat;
                        self.settings.longitude=lon;
                        self.settings.weather_city=if city.is_empty(){crate::i18n::t("weather.custom",&self.settings.language).to_string()}else{city};
                        self.settings.weather_configured=true;
                        self.weather_custom_open=false;
                        self.morph=None;self.layout();
                        self.save();
                        self.make_inputs();
                        self.refresh_weather();
                        self.invalidate();
                    },
                    _=>self.notice(crate::i18n::t("weather.invalid_coords",&self.settings.language))
                }
            },
            Action::WeatherRefresh=>self.refresh_weather(),
            Action::WeatherToggleMenu=>{self.weather_menu_open=!self.weather_menu_open;},
            Action::WeatherSetCity(city,lat,lon)=>{
                self.reset_weather_location();
                self.weather_custom_open=false;
                self.settings.weather_city=city.to_string();
                self.settings.latitude=lat;
                self.settings.longitude=lon;
                self.settings.weather_configured=true;
                self.save();
                self.make_inputs();
                self.refresh_weather();
                self.invalidate();
            },
            Action::WeatherToggleCustom=>{
                self.weather_menu_open=false;
                self.weather_custom_open=!self.weather_custom_open;
                self.morph=None;self.layout();
                self.make_inputs();
                if !self.weather_custom_open{self.drafts.retain(|(page,id),_|page!="Cuaca"||!(30..=32).contains(id));}
                self.invalidate();
            },
            Action::CheckUpdate=>{
                self.update_status=super::UpdateStatus::Checking;
                self.invalidate();
                let api=self.online.clone();
                self.job("Update",move||super::Event::Update(api.check_update()));
            },
            Action::DownloadUpdate=>{
                if let super::UpdateStatus::Available(ref rel) = self.update_status {
                    let zip_url = rel.assets.iter()
                        .find(|a| a.name.ends_with(".zip"))
                        .map(|a| a.browser_download_url.as_str())
                        .unwrap_or(rel.html_url.as_str());
                    crate::system::open_browser(zip_url);
                }
            },
            Action::OpenReleaseNotes=>{
                if let super::UpdateStatus::Available(ref rel) = self.update_status {
                    crate::system::open_browser(&rel.html_url);
                }
            },
            Action::OpenRepoUrl=>{
                crate::system::open_browser("https://github.com/arwankemis/mynotch");
            },
            Action::AiSave=>{let key=self.input(43);if !key.is_empty(){match crate::secrets::protect(&key){Ok(secret)=>self.settings.ai_key=secret,Err(e)=>{self.notice(e);return}}}self.settings.ai_endpoint=self.input(41).trim().into();self.settings.ai_model=self.input(42).trim().into();self.clear_input(43);self.notice(crate::i18n::t("notice.ai_saved", &self.settings.language));},Action::AiSend=>self.send_ai(),Action::AiClear=>self.ai_messages.clear(),
            Action::ToggleLyrics=>{
                self.lyric_mode=!self.lyric_mode;
                let from=self.geometry();
                self.morph=Some(ui::Morph::new(from,self.target_geometry()));
                self.update_window_region();
                self.start_morph_timer();
                self.invalidate();
            },
            Action::SelectMode(idx)=>match idx{
                0=>{self.tab="Musik".into();self.lyric_mode=false;self.set_expanded(true);},
                1=>{self.tab="Musik".into();self.lyric_mode=true;self.set_expanded(true);},
                2=>{self.settings.minimal=false;self.set_expanded(false);},
                3=>{self.settings.minimal=false;self.set_expanded(false);},
                4=>{self.settings.minimal=true;self.set_expanded(false);},
                _=>{}
            },
            Action::NoteAdd => {
                if self.input_page == "Catatan" {
                    let title = self.input(50);
                    let content = self.input(51);
                    if let Some(note) = self.state.sticky_notes.get_mut(self.state.selected_note) {
                        if !title.is_empty() || !content.is_empty() {
                            note.title = if title.trim().is_empty() { "Catatan Baru".into() } else { title };
                            note.content = content;
                            note.updated_at = Local::now().format("%Y-%m-%d %H:%M").to_string();
                        }
                    }
                    self.clear_input(50);
                    self.clear_input(51);
                    self.drafts.remove(&("Catatan".into(), 50));
                    self.drafts.remove(&("Catatan".into(), 51));
                }
                if self.state.sticky_notes.len() >= 20 {
                    self.notice(crate::i18n::t("notice.notes_max", &self.settings.language));
                } else {
                    let new_id = self.state.sticky_notes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                    let now_str = Local::now().format("%Y-%m-%d %H:%M").to_string();
                    let new_note = crate::sticky::StickyNote {
                        id: new_id,
                        title: "Catatan Baru".into(),
                        content: String::new(),
                        color_idx: 0,
                        updated_at: now_str,
                    };
                    self.state.sticky_notes.insert(0, new_note);
                    self.state.selected_note = 0;
                    self.trigger_stage1_temporary(5);
                    self.save();
                    self.make_inputs();
                    self.notice(crate::i18n::t("notice.note_added", &self.settings.language));
                    self.invalidate();
                }
            },
            Action::NoteSelect(idx) => {
                if self.input_page == "Catatan" {
                    let title = self.input(50);
                    let content = self.input(51);
                    if let Some(note) = self.state.sticky_notes.get_mut(self.state.selected_note) {
                        note.title = if title.trim().is_empty() { "Catatan Baru".into() } else { title };
                        note.content = content;
                        note.updated_at = Local::now().format("%Y-%m-%d %H:%M").to_string();
                    }
                    self.clear_input(50);
                    self.clear_input(51);
                    self.drafts.remove(&("Catatan".into(), 50));
                    self.drafts.remove(&("Catatan".into(), 51));
                }
                if idx < self.state.sticky_notes.len() {
                    self.state.selected_note = idx;
                    self.trigger_stage1_temporary(5);
                    self.save();
                    self.make_inputs();
                    self.invalidate();
                }
            },
            Action::NoteDelete(id) => {
                if self.input_page == "Catatan" {
                    self.clear_input(50);
                    self.clear_input(51);
                    self.drafts.remove(&("Catatan".into(), 50));
                    self.drafts.remove(&("Catatan".into(), 51));
                }
                self.state.sticky_notes.retain(|n| n.id != id);
                if self.state.selected_note >= self.state.sticky_notes.len() {
                    self.state.selected_note = self.state.sticky_notes.len().saturating_sub(1);
                }
                self.save();
                self.make_inputs();
                self.notice(crate::i18n::t("notice.note_deleted", &self.settings.language));
                self.invalidate();
            },
            Action::NoteColor(ci) => {
                if let Some(note) = self.state.sticky_notes.get_mut(self.state.selected_note) {
                    note.color_idx = ci % 5;
                    self.save();
                    self.invalidate();
                }
            },
            Action::NoteExport(kind) => {
                if self.input_page == "Catatan" {
                    let title = self.input(50);
                    let content = self.input(51);
                    if let Some(note) = self.state.sticky_notes.get_mut(self.state.selected_note) {
                        note.title = if title.trim().is_empty() { "Catatan Baru".into() } else { title };
                        note.content = content;
                        note.updated_at = Local::now().format("%Y-%m-%d %H:%M").to_string();
                    }
                    self.save();
                }
                if let Some(note) = self.state.sticky_notes.get(self.state.selected_note).cloned() {
                    match kind {
                        0 => {
                            let export_dir = crate::app::exports_directory(&self.store.root);
                            match crate::sticky::export_note_to_markdown(&export_dir, &note) {
                                Ok(path) => {
                                    unsafe {
                                        use windows::Win32::UI::Shell::ShellExecuteW;
                                        use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
                                        let path_w: Vec<u16> = path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
                                        ShellExecuteW(None, windows::core::w!("open"), PCWSTR(path_w.as_ptr()), None, None, SW_SHOW);
                                    }
                                    self.notice(crate::i18n::t("notice.export_md", &self.settings.language));
                                }
                                Err(e) => self.notice(format!("{} {}",crate::i18n::t("error.export", &self.settings.language), e)),
                            }
                        }
                        1 => {
                            let export_dir = crate::app::exports_directory(&self.store.root);
                            match crate::sticky::export_note_to_txt(&export_dir, &note) {
                                Ok(path) => {
                                    unsafe {
                                        use windows::Win32::UI::Shell::ShellExecuteW;
                                        use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
                                        let path_w: Vec<u16> = path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
                                        ShellExecuteW(None, windows::core::w!("open"), PCWSTR(path_w.as_ptr()), None, None, SW_SHOW);
                                    }
                                    self.notice(crate::i18n::t("notice.export_txt", &self.settings.language));
                                }
                                Err(e) => self.notice(format!("{} {}",crate::i18n::t("error.export", &self.settings.language), e)),
                            }
                        }
                        2 => {
                            let full_text = format!("{}\r\n\r\n{}", note.title, note.content);
                            if crate::clipboard::set_clipboard_text(&full_text) {
                                self.last_clipboard_seq = crate::clipboard::get_sequence_number();
                                self.notice(crate::i18n::t("notice.note_copied", &self.settings.language));
                            } else {
                                self.notice(crate::i18n::t("notice.clip_failed", &self.settings.language));
                            }
                        }
                        3 => {
                            unsafe {
                                use windows::Win32::UI::Shell::ShellExecuteW;
                                use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
                                let protocol = windows::core::w!("ms-stickynotes:");
                                let res = ShellExecuteW(None, windows::core::w!("open"), protocol, None, None, SW_SHOW);
                                if res.0 as isize > 32 {
                                    self.notice(crate::i18n::t("notice.windows_sticky_open", &self.settings.language));
                                } else {
                                    self.notice(crate::i18n::t("notice.windows_sticky_missing", &self.settings.language));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            Action::NotePopout => {
                if self.input_page == "Catatan" {
                    let title = self.input(50);
                    let content = self.input(51);
                    if let Some(note) = self.state.sticky_notes.get_mut(self.state.selected_note) {
                        note.title = if title.trim().is_empty() { "Catatan Baru".into() } else { title };
                        note.content = content;
                        note.updated_at = Local::now().format("%Y-%m-%d %H:%M").to_string();
                    }
                    self.save();
                }
                if let Some(note) = self.state.sticky_notes.get(self.state.selected_note) {
                    super::spawn_floating_sticky_note(note.id, &note.title, &note.content, note.color_idx, self.hwnd);
                    self.notice(crate::i18n::t("notice.note_popout", &self.settings.language));
                }
            },
        }
        self.sync_weather_lifecycle();
        for module in MODULES{if !self.settings.enabled(module){self.cancel_job(module);}}
        if !self.settings.enabled("Musik"){self.cancel_job("Artwork");self.cover=None;self.cover_key=0;self.lyrics_key.clear();self.plain_lyrics.clear();self.lyric_offset=0;}
        self.sync_foreground();self.sync_visualizer();if !transient{self.save();}self.clock_timer();self.invalidate();
    }
    fn media_command(&mut self,command:impl FnOnce(&MediaService)->Result<(),String>){if let Some(media)=&self.media{if let Err(e)=command(media){self.notice(e);}}else{self.notice(crate::i18n::t("music.open_player_first", &self.settings.language));}}
    fn refresh_agenda(&mut self){if !self.settings.enabled("Kalender"){return}let feed=match crate::secrets::unprotect(&self.settings.calendar_feed){Ok(feed)=>feed,Err(e)=>{self.notice(e);return}};if feed.is_empty(){self.cancel_job("Kalender");self.agenda.clear();self.notice(crate::i18n::t("notice.calendar_feed_removed", &self.settings.language));return}if self.jobs.contains_key("Kalender"){return}let api=self.online.clone();self.job("Kalender",move||Event::Agenda(api.agenda(&feed)));}
    fn send_ai(&mut self){if self.jobs.contains_key("AI")||self.fullscreen||!self.settings.enabled("AI"){return}let text=self.input(40).trim().to_owned();if text.is_empty(){return}let key=match crate::secrets::unprotect(&self.settings.ai_key){Ok(key)if!key.is_empty()=>key,_=>{self.notice(crate::i18n::t("notice.ai_key_missing", &self.settings.language));return}};if self.settings.ai_model.trim().is_empty(){self.notice(crate::i18n::t("notice.ai_model_missing", &self.settings.language));return}self.ai_messages.push(crate::online::ChatMessage{role:"user".into(),content:text});self.clear_input(40);let api=self.online.clone();let settings=self.settings.clone();let messages=self.ai_messages.clone();self.job("AI",move||Event::Ai(api.chat(&settings.ai_provider,&settings.ai_endpoint,&settings.ai_model,&key,&messages)));}

        pub(super) fn hit(&mut self, x: f32, y: f32, w: f32, h: f32, action: Action) {
        if self.hit_enabled {
            self.hits.push(Hit { rect: render::rect(x, y, w, h), action });
        }
    }



    fn draw_button(&mut self, p: &mut Painter, label: &str, x: f32, y: f32, w: f32, h: f32, action: Action) { unsafe {
        let hover = self.pointer.is_some_and(|(px, py)| px >= x && px <= x + w && py >= y && py <= y + h);
        p.fill(x, y, w, h, 6., if hover { 0x2e2e38 } else { 0x22222b });
        p.stroke(x, y, w, h, 6., if hover { 0x4e4e5d } else { 0x33333f }, 1.);
        p.text(label, x + 8., y + 7., w - 16., 16., 11., render::TEXT, false);
        self.hit(x, y, w, h, action);
    }}

    /// Community mods are listed in Settings → Mod. The manifest list is cached so painting never
    /// touches the disk; the cache is dropped on install/uninstall/delete.
    pub(super) fn mods(&mut self) -> Vec<crate::mods::ModManifest> {
        if self.mods_cache.is_none() {
            let mut list = crate::mods::available_mods(&self.store.root);
            list.sort_by(|a, b| {
                let rank = |m: &crate::mods::ModManifest| match m.id.as_str() { "sticky_notes" => 0u8, "weather" => 1, _ => 2 };
                rank(a).cmp(&rank(b)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            self.mods_cache = Some(list);
        }
        self.mods_cache.clone().unwrap_or_default()
    }
    pub(super) fn invalidate_mods(&mut self) { self.mods_cache = None; }

    /// Community mod currently being shown, whether via Settings → Buka or via a tab the user
    /// opted into. The Lua cache is keyed by the manifest id, so both routes share it.
    pub(super) fn active_mod_id(&mut self) -> Option<String> {
        if let Some(id) = self.mod_view.clone() {
            return Some(id);
        }
        let tab = self.tab.clone();
        if self.settings.mod_tabs.iter().any(|t| t == &tab) {
            return self.mods().into_iter().find(|m| m.tab_name == tab).map(|m| m.id);
        }
        None
    }

    /// Runs the Lua mod once and stores the result for painting. Called from action handling
    /// and the 1 s tick — never from `paint()`.
    pub(super) fn refresh_mod_output(&mut self) {
        let Some(id) = self.active_mod_id() else {
            self.mod_output = None;
            return
        };
        let Some(manifest) = self.mods().into_iter().find(|m| m.id == id) else {
            self.mod_output = None;
            return
        };
        if manifest.mod_type != crate::mods::ModType::Lua {
            self.mod_output = None;
            return
        }
        let button = self.last_mod_button_clicked.take();
        let mod_dir = crate::mods::get_mod_dir(&self.store.root, &manifest.id);
        let script = mod_dir.join("main.lua");
        let config = std::fs::read(mod_dir.join("config.json"))
            .ok()
            .filter(|bytes| bytes.len() <= 64 * 1024)
            .and_then(|bytes| {
                let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(&bytes);
                serde_json::from_slice::<serde_json::Value>(bytes).ok()
            });
        self.mod_http_ttl = Duration::from_secs(
            config.as_ref()
                .and_then(|value| value.get("refresh_seconds"))
                .and_then(|value| value.as_u64())
                .unwrap_or(60)
                .clamp(15, 600),
        );
        let now = Instant::now();
        let http_cache: std::collections::BTreeMap<String, Result<String, String>> = self.mod_http_cache.iter()
            .filter(|(_, (fetched_at, _))| now.duration_since(*fetched_at) < self.mod_http_ttl)
            .map(|(url, (_, result))| (url.clone(), result.clone()))
            .collect();
        let http_allowed = self.settings.network_mods.iter().any(|mod_id| mod_id == &manifest.id);
        let output = crate::mod_script::run_lua_mod_with_config(
            &script,
            button.as_deref(),
            config.as_ref(),
            http_allowed,
            &http_cache,
        );
        // Side effects only on a click or the first run of this mod; a periodic refresh must not
        // reopen the browser or re-copy to the clipboard every second.
        let first_run = self.mod_output.as_ref().is_none_or(|(cached, _)| cached != &manifest.id);
        if let Ok(value) = &output {
            if button.is_some() || first_run {
                for toast in value.toasts.clone() { self.notice(toast); }
                for url in &value.urls_to_open { crate::system::open_browser(url); }
                for text in &value.clipboard_copies {
                    crate::clipboard::set_clipboard_text(text);
                    self.notice(crate::i18n::t("notice.clip_copied", &self.settings.language));
                }
            }
            // One background fetch at a time; the completion event re-runs the Lua with the body.
            if http_allowed && !self.jobs.contains_key("ModHttp") {
                if let Some(url) = value.http_requests.first().cloned() {
                    self.job("ModHttp", move || {
                        let result = crate::net::request(
                            "GET",
                            &url,
                            &[
                                ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36"),
                                ("Accept", "application/json"),
                            ],
                            &[],
                        ).map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
                        Event::ModHttp(url, result)
                    });
                }
            }
        }
        self.mod_output = Some((manifest.id, output));
    }

    /// Stage 2 view for a community mod, opened from Settings → Mod. It owns its own header and
    /// back button, so community mods never take a slot in the main tab strip.
    pub(super) fn draw_mod_view(&mut self, p: &mut Painter) { unsafe {
        let Some(id) = self.mod_view.clone() else { return };
        let Some(manifest) = self.mods().into_iter().find(|m| m.id == id) else {
            self.mod_view = None;
            self.settings_open = true;
            self.settings_cat = 5;
            self.notice(crate::i18n::t("mods.missing", &self.settings.language));
            let from = self.geometry();
            self.morph = Some(ui::Morph::new(from, self.target_geometry()));
            self.start_morph_timer();
            return
        };
        let lang = self.settings.language.clone();
        self.draw_button(p, crate::i18n::t("mods.back", &lang), 18., 12., 92., 26., Action::CloseModView);
        p.outline_icon(&manifest.icon, 122., 16., 18., 0x6ee7b7);
        p.text_ellipsis(&manifest.name, 148., 9., self.width - 166., 18., 13., render::TEXT, true);
        let kind = match manifest.mod_type {
            crate::mods::ModType::Lua => "Lua",
            crate::mods::ModType::Declarative => "JSON",
            crate::mods::ModType::Builtin => crate::i18n::t("mods.type_builtin", &lang),
        };
        p.text_ellipsis(&format!("v{} · {}", manifest.version, kind), 148., 27., self.width - 166., 14., 9.5, MUTED, false);
        p.text_ellipsis(&manifest.description, 18., 47., self.width - 36., 16., 10., MUTED, false);
        p.fill(18., 67., self.width - 36., 1., 0., 0x222229);
        self.draw_mod_body(p, &manifest, 77.);
    }}

    /// Renders a community mod that the user opted into the Stage 2 tab strip.
    fn draw_mod_tab(&mut self, p: &mut Painter) -> bool { unsafe {
        let tab = self.tab.clone();
        if !self.settings.mod_tabs.iter().any(|t| t == &tab) { return false }
        let Some(manifest) = self.mods().into_iter().find(|m| m.tab_name == tab) else { return false };
        p.outline_icon(&manifest.icon, 24., 38., 20., 0x6ee7b7);
        p.text_ellipsis(&manifest.name, 52., 34., self.width - 76., 18., 13., render::TEXT, true);
        p.text_ellipsis(&manifest.description, 52., 53., self.width - 76., 14., 9.5, MUTED, false);
        p.fill(24., 74., self.width - 48., 1., 0., 0x222229);
        self.draw_mod_body(p, &manifest, 82.);
        true
    }}

    fn draw_mod_body(&mut self, p: &mut Painter, manifest: &crate::mods::ModManifest, start_y: f32) { unsafe {
        let margin = 18.;
        let content_w = self.width - margin * 2.;
        match manifest.mod_type {
            crate::mods::ModType::Declarative => {
                let card_w = (content_w - 12.) / 2.;
                let card_h = 68.;
                for (i, card) in manifest.cards.iter().enumerate() {
                    let x = margin + (i % 2) as f32 * (card_w + 12.);
                    let y = start_y + (i / 2) as f32 * (card_h + 10.);
                    p.fill(x, y, card_w, card_h, 8., 0x16161d);
                    p.stroke(x, y, card_w, card_h, 8., 0x2b2b36, 1.);
                    p.text(&card.title, x + 12., y + 8., card_w - 24., 16., 10., MUTED, false);
                    p.text(&card.value, x + 12., y + 24., card_w - 24., 24., 14., render::TEXT, false);
                    if !card.subtext.is_empty() {
                        p.text(&card.subtext, x + 12., y + 48., card_w - 24., 14., 9., MUTED, false);
                    }
                }
                let btn_w = (content_w - 24.) / 3.;
                let btn_start_y = if manifest.cards.is_empty() {
                    start_y
                } else {
                    start_y + ((manifest.cards.len() + 1) / 2) as f32 * (card_h + 10.) + 2.
                };
                for (i, btn) in manifest.buttons.iter().enumerate() {
                    let x = margin + (i % 3) as f32 * (btn_w + 12.);
                    let y = btn_start_y + (i / 3) as f32 * 38.;
                    let action = if btn.action == "open_url" && !btn.target.is_empty() {
                        Action::ModButtonClick(format!("url:{}", btn.target))
                    } else if btn.action == "toast" && !btn.target.is_empty() {
                        Action::ModButtonClick(format!("toast:{}", btn.target))
                    } else if btn.action == "copy" && !btn.target.is_empty() {
                        Action::ModButtonClick(format!("copy:{}", btn.target))
                    } else {
                        Action::ModButtonClick(btn.label.clone())
                    };
                    self.draw_button(p, &btn.label, x, y, btn_w, 30., action);
                }
            }
            crate::mods::ModType::Lua => {
                // The VM never runs here: paint only renders the last result computed in
                // `refresh_mod_output()` (open, button click, 1 s tick).
                match self.mod_output.clone().filter(|(cached, _)| cached == &manifest.id) {
                    Some((_, Ok(output))) => {
                        for t in &output.texts {
                            p.text(&t.text, t.x, t.y, 400., 20., 11., render::TEXT, false);
                        }
                        let mut body_y = start_y;
                        if let Some(headline) = &output.headline {
                            body_y = draw_headline(p, headline, body_y, self.width);
                        }
                        if output.ranked_left.is_some() || output.ranked_right.is_some() {
                            draw_ranked(p, output.ranked_left.as_ref(), output.ranked_right.as_ref(), body_y, self.width);
                            return;
                        }
                        let card_w = (content_w - 12.) / 2.;
                        let card_h = 68.;
                        for (i, card) in output.cards.iter().enumerate() {
                            let x = margin + (i % 2) as f32 * (card_w + 12.);
                            let y = body_y + (i / 2) as f32 * (card_h + 10.);
                            p.fill(x, y, card_w, card_h, 8., 0x16161d);
                            p.stroke(x, y, card_w, card_h, 8., 0x2b2b36, 1.);
                            p.text(&card.title, x + 12., y + 8., card_w - 24., 16., 10., MUTED, false);
                            p.text(&card.value, x + 12., y + 24., card_w - 24., 24., 14., render::TEXT, false);
                            if let Some(sub) = &card.subtext {
                                p.text(sub, x + 12., y + 48., card_w - 24., 14., 9., MUTED, false);
                            }
                        }
                        let btn_w = (content_w - 24.) / 3.;
                        let btn_start_y = if output.cards.is_empty() {
                            body_y
                        } else {
                            body_y + ((output.cards.len() + 1) / 2) as f32 * (card_h + 10.) + 2.
                        };
                        for (i, btn) in output.buttons.iter().enumerate() {
                            let x = margin + (i % 3) as f32 * (btn_w + 12.);
                            let y = btn_start_y + (i / 3) as f32 * 38.;
                            self.draw_button(p, &btn.label, x, y, btn_w, 30., Action::ModButtonClick(btn.id.clone()));
                        }
                    }
                    Some((_, Err(err))) => {
                        p.fill(margin, start_y + 6., content_w, 60., 8., 0x2b1515);
                        p.stroke(margin, start_y + 6., content_w, 60., 8., 0x7f2323, 1.);
                        p.text(crate::i18n::t("mods.script_error", &self.settings.language), margin + 12., start_y + 14., content_w - 24., 20., 12., 0xff8888, false);
                        p.text_ellipsis(&err, margin + 12., start_y + 36., content_w - 24., 20., 10., 0xddaaaa, false);
                    }
                    None => {
                        p.text_ellipsis(crate::i18n::t("music.loading_lyrics", &self.settings.language), margin, start_y + 6., content_w, 20., 11., MUTED, false);
                    }
                }
            }
            crate::mods::ModType::Builtin => {}
        }
    }}
}

const LANE_GREEN: u32 = 0x6ee7b7;
const LANE_RED: u32 = 0xf87171;
const LANE_FAINT: u32 = 0x7f7f8a;

/// Summary band for a market-style mod: label, big value, signed delta, and a right-aligned
/// meta line plus an optional badge. Returns the y coordinate below the band.
unsafe fn draw_headline(p: &mut Painter, headline: &LuaHeadline, y: f32, width: f32) -> f32 { unsafe {
    let margin = 18.;
    let content_w = width - margin * 2.;
    p.text_ellipsis(&headline.label, margin, y, content_w - 150., 12., 9., LANE_FAINT, false);
    if !headline.badge.trim().is_empty() {
        let badge_w = p.measure_text(&headline.badge, 7.5, false) + 12.;
        let badge_x = width - margin - badge_w;
        p.stroke(badge_x, y - 1., badge_w, 14., 4., 0x3a3a45, 1.);
        p.text_ellipsis(&headline.badge, badge_x + 5., y + 1., badge_w - 9., 11., 7.5, LANE_FAINT, false);
    }
    let value_y = y + 11.;
    p.text(&headline.value, margin, value_y, content_w, 24., 20., render::TEXT, true);
    let value_w = p.measure_text(&headline.value, 20., true);
    if !headline.delta.trim().is_empty() {
        let color = if headline.delta.trim_start().starts_with('-') { LANE_RED } else { LANE_GREEN };
        p.text_ellipsis(&headline.delta, margin + value_w + 10., value_y + 9., 150., 14., 10.5, color, true);
    }
    if !headline.meta.trim().is_empty() {
        let meta_w = p.measure_text(&headline.meta, 8.5, false).min(210.);
        p.text_ellipsis(&headline.meta, width - margin - meta_w, y + 19., meta_w, 12., 8.5, MUTED, false);
    }
    y + 44.
}}

/// Two ranked lanes separated by a hairline. Rows are fixed at 32 px so five entries fit the
/// Stage 2 panel under the headline.
unsafe fn draw_ranked(p: &mut Painter, left: Option<&LuaRanked>, right: Option<&LuaRanked>, y: f32, width: f32) { unsafe {
    let margin = 18.;
    let gap = 14.;
    let content_w = width - margin * 2.;
    let both = left.is_some() && right.is_some();
    let lane_w = if both { (content_w - gap - 1.) / 2. } else { content_w };
    let count = left.map(|l| l.rows.len()).unwrap_or(0).max(right.map(|r| r.rows.len()).unwrap_or(0)).min(5);
    let lane_h = 18. + count as f32 * 32.;
    if let Some(lane) = left {
        draw_ranked_lane(p, lane, margin, y, lane_w, true);
    }
    if both {
        let divider_x = margin + lane_w + gap * 0.5;
        p.fill_alpha(divider_x, y + 2., 1., lane_h, 0., 0xffffff, 0.07);
    }
    if let Some(lane) = right {
        let x = if left.is_some() { margin + lane_w + gap } else { margin };
        draw_ranked_lane(p, lane, x, y, lane_w, false);
    }
}}

unsafe fn draw_ranked_lane(p: &mut Painter, lane: &LuaRanked, x: f32, y: f32, width: f32, positive: bool) { unsafe {
    let color = if positive { LANE_GREEN } else { LANE_RED };
    let arrow = if positive { "▲" } else { "▼" };
    let title = format!("{arrow} {}", lane.title.to_uppercase());
    p.text_ellipsis(&title, x, y, width - 112., 12., 9., color, true);
    p.text_ellipsis(&lane.meta, x + width - 108., y + 1., 108., 12., 8., color, false);
    if lane.rows.is_empty() {
        p.text("Tidak ada data", x, y + 26., width, 16., 9., MUTED, false);
        return;
    }
    let max = lane.rows.iter().map(|r| r.change.abs()).fold(0.0_f64, f64::max).max(0.0001);
    let name_w = (width - 22. - 68.).max(40.);
    for (i, row) in lane.rows.iter().take(5).enumerate() {
        let row_y = y + 18. + i as f32 * 32.;
        p.text(&format!("{:02}", i + 1), x + 2., row_y + 9., 16., 12., 8.5, LANE_FAINT, false);
        p.text_ellipsis(&row.ticker, x + 22., row_y + 3., name_w, 14., 11.5, render::TEXT, true);
        p.text_ellipsis(&row.name, x + 22., row_y + 17., name_w, 12., 8.5, MUTED, false);
        let price_w = p.measure_text(&row.price, 10.5, false);
        p.text(&row.price, x + width - price_w - 1., row_y + 3., price_w + 2., 14., 10.5, render::TEXT, false);
        let change = format!("{:+.2}%", row.change).replace('.', ",");
        let change_w = p.measure_text(&change, 9.5, true);
        // A filled lane can contain the opposite sign, so colour the value by its own direction.
        let change_color = if row.change >= 0.0 { LANE_GREEN } else { LANE_RED };
        p.text(&change, x + width - change_w - 1., row_y + 17., change_w + 2., 12., 9.5, change_color, true);
        p.fill_alpha(x + 22., row_y + 30., width - 22., 2., 1., 0xffffff, 0.045);
        let bar_w = (((row.change.abs() / max) as f32) * (width - 22.)).max(6.);
        p.fill_alpha(x + 22., row_y + 30., bar_w, 2., 1., change_color, 0.32);
    }
}}
