use super::*;
use render::{TEXT,MUTED,ACCENT,CARD,DIM};
use chrono::Datelike;

impl Shell {
    pub(super) fn draw_page(&mut self,p:&mut Painter){unsafe{
        match self.tab.as_str(){
            "Musik"=>self.music_page(p),
            "Kalender"=>self.calendar_page(p),
            "Reminder"=>self.reminder_page(p),
            "Clipboard"=>self.clipboard_page(p),
            _=>{p.text("Pilih modul melalui Pengaturan.",24.,140.,620.,80.,18.,MUTED,false);}
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
        // 1. Track Content (Artwork 60x60 from y=44 to y=104)
        let art_x=24.;let art_y=44.;let art_size=60.;
        let key=if self.cover.is_some(){self.cover_key}else{0};
        if !p.cover(self.cover.as_ref(),key,art_x,art_y,art_size,false){
            p.fill(art_x,art_y,art_size,art_size,8.,0x16222b);
            p.text("♫",art_x+19.,art_y+13.,24.,30.,22.,0x8dc4dd,false);
        }

        // Title (bold 14px white)
        let title=if self.track.title.is_empty(){"Belum ada musik"}else{&self.track.title};
        p.text_ellipsis(title,94.,44.,self.width-138.,18.,14.,0xffffff,true);

        // Media source app icon (Spotify, etc) placed at far right of title row
        p.icon_media_app(self.width-28.,52.,&self.track.source_app,self.track.playing);

        // Artist (12px #9ea3a8)
        let artist=if self.track.artist.is_empty(){"Tidak ada artis".into()}else{self.track.artist.clone()};
        p.text_ellipsis(&artist,94.,65.,self.width-138.,16.,12.,0x9ea3a8,false);

        // Album (11px #6e7681)
        let album=if self.track.album.is_empty(){"Single / Album tidak diketahui".into()}else{self.track.album.clone()};
        p.text_ellipsis(&album,94.,85.,self.width-138.,16.,11.,0x6e7681,false);

        // 2. Scrubber (At y=114, below the artwork)
        let position=self.media.as_ref().map(|m|m.position_seconds()).unwrap_or(self.track.position_seconds);
        let duration=self.track.duration_seconds;
        let ratio=if duration>0.{(position/duration).clamp(0.,1.)}else{0.};

        let scrub_y=114.;
        p.text(&time_label(position),24.,scrub_y,34.,16.,11.,0xffffff,false);
        let track_x=62.;let track_w=self.width-124.;
        p.fill(track_x,scrub_y+6.,track_w,4.,2.,0x3e4948);
        if ratio>0.{
            p.fill(track_x,scrub_y+6.,track_w*ratio as f32,4.,2.,0xffffff);
        }
        if self.track.can_seek{
            self.hits.push(Hit{rect:render::rect(track_x,scrub_y,track_w,18.),action:Action::SeekPointer});
        }
        let remaining=(duration-position).max(0.);
        p.text(&format!("-{}",time_label(remaining)),self.width-58.,scrub_y,40.,16.,11.,0xffffff,false);

        // 3. Transport Controls (At y=138, below scrubber)
        let trans_y=138.;
        // Lyrics toggle quote button at x=24
        self.transport_btn(p,"LYRICS",24.,trans_y,28.,28.,Action::ToggleLyrics,self.lyric_mode);
        // Centered Previous / Play / Next
        let mid_x=self.width/2.;
        self.transport_btn(p,"⏮",mid_x-52.,trans_y,28.,28.,Action::Previous,false);
        let play_symbol=if self.track.playing{"⏸"}else{"▶"};
        self.transport_btn(p,play_symbol,mid_x-14.,trans_y,28.,28.,Action::Play,false);
        self.transport_btn(p,"⏭",mid_x+24.,trans_y,28.,28.,Action::Next,false);
    }}

    fn draw_mockup_lyrics(&mut self,p:&mut Painter){unsafe{
        let w=self.width;
        // Left Half: Track Content (x = 24..230)
        let art_x=24.;let art_y=44.;let art_size=60.;
        let key=if self.cover.is_some(){self.cover_key}else{0};
        if !p.cover(self.cover.as_ref(),key,art_x,art_y,art_size,false){
            p.fill(art_x,art_y,art_size,art_size,8.,0x16222b);
            p.text("♫",art_x+19.,art_y+13.,24.,30.,22.,0x8dc4dd,false);
        }

        let title=if self.track.title.is_empty(){"Belum ada musik"}else{&self.track.title};
        p.text_ellipsis(title,94.,44.,120.,18.,14.,0xffffff,true);
        p.icon_media_app(224.,52.,&self.track.source_app,self.track.playing);

        let artist=if self.track.artist.is_empty(){"Tidak ada artis".into()}else{self.track.artist.clone()};
        p.text_ellipsis(&artist,94.,65.,126.,16.,12.,0x9ea3a8,false);

        let album=if self.track.album.is_empty(){"Single / Album".into()}else{self.track.album.clone()};
        p.text_ellipsis(&album,94.,85.,126.,16.,11.,0x6e7681,false);

        // Scrubber
        let position=self.media.as_ref().map(|m|m.position_seconds()).unwrap_or(self.track.position_seconds);
        let duration=self.track.duration_seconds;
        let ratio=if duration>0.{(position/duration).clamp(0.,1.)}else{0.};

        let scrub_y=114.;
        p.text(&time_label(position),24.,scrub_y,30.,14.,10.,0xffffff,false);
        let track_x=58.;let track_w=128.;
        p.fill(track_x,scrub_y+5.,track_w,4.,2.,0x3e4948);
        if ratio>0.{
            p.fill(track_x,scrub_y+5.,track_w*ratio as f32,4.,2.,0xffffff);
        }
        if self.track.can_seek{
            self.hits.push(Hit{rect:render::rect(track_x,scrub_y,track_w,16.),action:Action::SeekPointer});
        }
        let remaining=(duration-position).max(0.);
        p.text(&format!("-{}",time_label(remaining)),190.,scrub_y,36.,14.,10.,0xffffff,false);

        // Transport Controls
        let trans_y=138.;
        self.transport_btn(p,"LYRICS",24.,trans_y,28.,28.,Action::ToggleLyrics,true);
        self.transport_btn(p,"⏮",64.,trans_y,28.,28.,Action::Previous,false);
        let play_symbol=if self.track.playing{"⏸"}else{"▶"};
        self.transport_btn(p,play_symbol,104.,trans_y,28.,28.,Action::Play,false);
        self.transport_btn(p,"⏭",144.,trans_y,28.,28.,Action::Next,false);

        // Subtle vertical divider (x = 238)
        p.fill_alpha(238.,44.,1.,122.,0.,0xffffff,0.12);

        // Right Half: Synchronized Lyrics (x = 252..w-20)
        let lyrics_x=252.;
        let lyrics_w=w-272.;
        if self.lyrics.is_empty(){
            p.text(if self.plain_lyrics.is_empty(){"Lirik belum tersedia untuk lagu ini."}else{&self.plain_lyrics},lyrics_x,70.,lyrics_w,50.,13.,0x72777b,false);
        }else{
            let active=crate::lyrics::active_line(&self.lyrics,position);
            let curr=active.unwrap_or(0);
            if curr>0{
                let prev_line=&self.lyrics[curr-1];
                p.text_ellipsis(&prev_line.text,lyrics_x,46.,lyrics_w,24.,13.,0x72777b,false);
                if self.track.can_seek{
                    self.hits.push(Hit{rect:render::rect(lyrics_x,44.,lyrics_w,26.),action:Action::Lyric(curr-1)});
                }
            }
            let active_line=&self.lyrics[curr];
            p.text_ellipsis(&active_line.text,lyrics_x,74.,lyrics_w,28.,15.,0xffffff,true);
            if self.track.can_seek{
                self.hits.push(Hit{rect:render::rect(lyrics_x,72.,lyrics_w,30.),action:Action::Lyric(curr)});
            }
            if curr+1<self.lyrics.len(){
                let next_line=&self.lyrics[curr+1];
                p.text_ellipsis(&next_line.text,lyrics_x,104.,lyrics_w,24.,13.,0x72777b,false);
                if self.track.can_seek{
                    self.hits.push(Hit{rect:render::rect(lyrics_x,102.,lyrics_w,26.),action:Action::Lyric(curr+1)});
                }
            }
            if curr+2<self.lyrics.len(){
                let next2_line=&self.lyrics[curr+2];
                p.text_ellipsis(&next2_line.text,lyrics_x,130.,lyrics_w,20.,11.,0x555555,false);
            }
        }
    }}


    fn calendar_page(&mut self,p:&mut Painter){unsafe{
        if self.width<320.{return}
        const MONTHS:[&str;12]=["Januari","Februari","Maret","April","Mei","Juni","Juli","Agustus","September","Oktober","November","Desember"];

        // LEFT COLUMN (Month Grid): x = 24..296
        p.text(&format!("{} {}",MONTHS[(self.month-1)as usize],self.year),24.,44.,160.,20.,15.,0xffffff,true);
        self.button_icon(p,"‹",236.,42.,24.,24.,Action::Month(-1));
        self.button_icon(p,"›",266.,42.,24.,24.,Action::Month(1));

        let col_w=272./7.;
        let day_names=["Sen","Sel","Rab","Kam","Jum","Sab","Min"];
        for(i,name)in day_names.iter().enumerate(){
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

            if is_selected{
                p.fill(x+4.,y-1.,col_w-8.,22.,6.,0xeaeaea);
                p.text_centered(&day.to_string(),x+4.,y-1.,col_w-8.,22.,12.,0x090909,1.0,true);
            }else{
                if is_today{
                    p.stroke(x+4.,y-1.,col_w-8.,22.,6.,0x555555,1.0);
                }
                let color=if is_holiday||is_sunday{
                    0xff7d7d
                }else if is_leave&&self.settings.collective_leave{
                    0xffa46b
                }else{
                    0xc6c6cc
                };
                p.text_centered(&day.to_string(),x+4.,y-1.,col_w-8.,22.,12.,color,1.0,is_today);
            }

            // Dot indicators below date
            if is_holiday{
                p.fill(x+col_w*0.5-1.5,y+18.,3.,3.,1.5,0xff7d7d);
            }else if is_leave&&self.settings.collective_leave{
                p.fill(x+col_w*0.5-1.5,y+18.,3.,3.,1.5,0xffa46b);
            }

            self.hits.push(Hit{rect:render::rect(x,y-2.,col_w,24.),action:Action::Day(day)});
        }

        // Calendar legend at y = 232
        p.fill(24.,236.,4.,4.,2.,0xff7d7d);
        p.text("Libur nasional",32.,232.,80.,14.,10.,0x77777c,false);
        p.fill(124.,236.,4.,4.,2.,0xffa46b);
        p.text("Cuti bersama",132.,232.,80.,14.,10.,0x77777c,false);

        // Footer at y = 254
        p.text(if self.year==2026{"Indonesia · data libur 2026"}else{"Data libur tahun ini belum tersedia"},24.,254.,184.,16.,9.,0x909096,false);
        let leave_label=if self.settings.collective_leave{"☑ Cuti bersama"}else{"☐ Cuti bersama"};
        p.text(leave_label,200.,254.,90.,16.,9.,0x9c9ca3,false);
        self.hits.push(Hit{rect:render::rect(198.,250.,94.,20.),action:Action::CalendarLeaveToggle});

        // Vertical Divider line between columns (x = 308)
        p.fill_alpha(308.,44.,1.,226.,0.,0xffffff,0.08);

        // RIGHT COLUMN (Date Detail & Upcoming Holiday): x = 324..516
        if let Some(date)=NaiveDate::from_ymd_opt(self.year,self.month,self.day){
            const DAYS:[&str;7]=["SENIN","SELASA","RABU","KAMIS","JUMAT","SABTU","MINGGU"];
            let day_idx=date.weekday().num_days_from_monday() as usize;
            p.text(DAYS[day_idx],324.,44.,160.,14.,10.,0xffa46b,true);
            p.text(&self.day.to_string(),324.,56.,80.,46.,42.,0xffffff,true);
            p.text(&format!("{} {}",MONTHS[(self.month-1)as usize],self.year),324.,104.,160.,16.,11.,0x9b9ba0,false);

            let holiday=self.holidays.iter().find(|h|h.date==date);
            let status_text=if self.year!=2026{"Data libur belum tersedia".into()}else if let Some(h)=holiday{
                if h.collective_leave{format!("Cuti bersama · {}",h.name)}else{format!("Libur nasional · {}",h.name)}
            }else if date==today{
                "Hari ini · bukan hari libur".into()
            }else if date.weekday().num_days_from_monday()==6{
                "Minggu · akhir pekan".into()
            }else{
                "Bukan hari libur nasional".into()
            };
            let status_color=if holiday.is_some_and(|h|!h.collective_leave){0xff7d7d}else if holiday.is_some(){0xffa46b}else{0x88888e};
            p.text_ellipsis(&status_text,324.,122.,188.,18.,11.,status_color,false);

            // Button "Hari ini"
            p.fill(324.,146.,64.,24.,6.,0x222222);
            p.stroke(324.,146.,64.,24.,6.,0x333333,1.0);
            p.text_centered("Hari ini",324.,146.,64.,24.,10.,0xdddddd,1.0,false);
            self.hits.push(Hit{rect:render::rect(324.,146.,64.,24.),action:Action::CalendarToday});

            // Divider
            p.fill_alpha(324.,178.,188.,1.,0.,0xffffff,0.08);

            // Upcoming Holiday
            p.text("LIBUR NASIONAL BERIKUTNYA",324.,186.,188.,14.,8.,0x88888e,true);

            // Find next upcoming holiday strictly >= today
            let next_h=self.holidays.iter().filter(|h|h.date>=today&&!h.collective_leave).min_by_key(|h|h.date);
            if let Some(nh)=next_h{
                let card_y=202.;
                p.fill(324.,card_y,188.,42.,8.,0x181517);
                p.stroke(324.,card_y,188.,42.,8.,0x292022,1.0);

                // Date badge
                p.fill(328.,card_y+3.,34.,36.,6.,0x2f171a);
                p.text_centered(&nh.date.day().to_string(),328.,card_y+4.,34.,18.,14.,0xff9a9a,1.0,true);
                let short_m=match nh.date.month(){1=>"JAN",2=>"FEB",3=>"MAR",4=>"APR",5=>"MEI",6=>"JUN",7=>"JUL",8=>"AGS",9=>"SEP",10=>"OKT",11=>"NOV",_=>"DES"};
                p.text_centered(short_m,328.,card_y+20.,34.,14.,8.,0xff9a9a,0.9,true);

                // Title & Date description
                p.text_ellipsis(&nh.name,368.,card_y+6.,124.,16.,11.,0xffffff,true);
                let day_name_id=match nh.date.weekday().num_days_from_monday(){0=>"Senin",1=>"Selasa",2=>"Rabu",3=>"Kamis",4=>"Jumat",5=>"Sabtu",_=>"Minggu"};
                let date_desc=format!("{}, {} {}",day_name_id,nh.date.day(),MONTHS[(nh.date.month()-1)as usize]);
                p.text_ellipsis(&date_desc,368.,card_y+22.,124.,14.,10.,0x83838a,false);

                // Arrow icon
                p.text("↗",494.,card_y+12.,14.,16.,12.,0x888888,false);

                // Clicking holiday card jumps to that date!
                let delta=nh.date.year()*12+nh.date.month() as i32 - (self.year*12+self.month as i32);
                self.hits.push(Hit{rect:render::rect(324.,card_y,188.,42.),action:Action::Month(delta)});
            }
        }
    }}


    fn music_control(&mut self,p:&mut Painter,label:&str,x:f32,y:f32,width:f32,action:Action,primary:bool,enabled:bool){if enabled{self.button(p,label,x,y,width,action,primary);}else{unsafe{p.fill(x,y,width,32.,9.,0x202c34);p.text(label,x+10.,y+6.,width-16.,22.,12.,0x637481,false);}}}
    fn clipboard_page(&mut self, p: &mut Painter) { unsafe {
        let w = self.width;
        let total_clips = self.state.clipboard_history.len();

        if self.selected_clip >= total_clips && total_clips > 0 {
            self.selected_clip = total_clips - 1;
        }

        // ==========================================
        // LEFT PANE: History List (x = 22..234, w = 212)
        // ==========================================
        let left_x = 22.;
        let left_w = 212.;

        p.text("RIWAYAT KLIP", left_x, 46., 90., 18., 10.5, 0x8a9291, true);
        let badge_text = format!("({})", total_clips);
        p.text(&badge_text, left_x + 84., 46., 40., 18., 10.0, 0x38bdf8, true);

        if total_clips > 0 {
            self.button_h(p, "Bersihkan", left_x + 140., 44., 72., 20., Action::ClearClips, false);
        }

        if total_clips == 0 {
            p.text("Belum ada riwayat.", left_x, 80., left_w, 20., 11., 0x6e7681, false);
            p.text("Teks yang Anda salin (Ctrl+C) di Windows akan otomatis tersimpan dan muncul di sini.", left_x, 104., left_w, 54., 10., 0x475569, false);
        } else {
            let start = self.clip_offset.min(total_clips.saturating_sub(1));
            let visible_items: Vec<(usize, crate::clipboard::ClipItem)> = self.state.clipboard_history
                .iter()
                .enumerate()
                .skip(start)
                .take(4)
                .map(|(idx, item)| (idx, item.clone()))
                .collect();

            for (i, (abs_idx, item)) in visible_items.into_iter().enumerate() {
                let item_y = 68. + i as f32 * 36.;
                let is_selected = abs_idx == self.selected_clip;
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

                let first_line = item.text.lines().next().unwrap_or("").trim();
                let display_title = if first_line.is_empty() { "(Teks kosong)" } else { first_line };
                p.text_ellipsis(display_title, left_x + 8., item_y + 3., left_w - 16., 15., 11.0, if is_selected { 0xffffff } else { 0xe2e8f0 }, is_selected);

                let meta = format!("{} · {} kar · {} baris", item.time, item.chars, item.lines);
                p.text(&meta, left_x + 8., item_y + 17., left_w - 16., 12., 9.0, if is_selected { 0x93c5fd } else { 0x64748b }, false);

                if self.hit_enabled {
                    self.hits.push(Hit {
                        rect: render::rect(left_x, item_y, left_w, 32.),
                        action: Action::SelectClip(abs_idx),
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

        if total_clips == 0 {
            p.text_centered("📋", right_x, 82., right_w, 28., 24., 0x38bdf8, 0.8, false);
            p.text_centered("Riwayat Clipboard Kosong", right_x, 114., right_w, 20., 13., 0xffffff, 1.0, true);
            p.text_centered("Salin teks (Ctrl+C) di mana saja,", right_x, 138., right_w, 16., 10.5, 0x8a9291, 1.0, false);
            p.text_centered("Notch akan otomatis menyimpannya di sini.", right_x, 154., right_w, 16., 10.5, 0x8a9291, 1.0, false);
        } else if let Some(item) = self.state.clipboard_history.get(self.selected_clip).cloned() {
            let title = format!("Klip #{} · {}", self.selected_clip + 1, item.time);
            p.text(&title, right_x, 46., 120., 18., 11.5, 0x38bdf8, true);

            let btn_w = 98.;
            let btn_x = w - 20. - btn_w;
            self.button_h(p, "📋 Salin Ulang", btn_x, 43., btn_w, 22., Action::CopyClip(self.selected_clip), true);

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
                let more_text = format!("+ {} baris lainnya...", all_lines.len() - max_lines);
                p.text(&more_text, right_x + 10., more_y, right_w - 20., 14., 9.5, 0x64748b, false);
            }

            let footer_y = 212.;
            let count_meta = format!("{} karakter · {} baris", item.chars, item.lines);
            p.text(&count_meta, right_x + 4., footer_y + 2., 140., 14., 9.5, 0x64748b, false);
            self.button_h(p, "🗑 Hapus", w - 20. - 68., footer_y, 68., 20., Action::DeleteClip(item.id), false);
        }
    }}

    pub(super) fn action(&mut self,action:Action){
        let transient=matches!(&action,Action::SettingsCategory(..)|Action::SettingsMenu(..)|Action::Settings|Action::Back|Action::Tab(..)|Action::ReminderMenu(..)|Action::ReminderSelect(..)|Action::ReminderStep(..));
        match action{
            Action::TogglePin=>{
                self.pinned=!self.pinned;
                self.notice(if self.pinned{"Notch disematkan (pin)"}else{"Sematan notch dilepas"});
                self.invalidate();
            },
            Action::ToStage0=>{
                self.hover_blocked_until=Instant::now()+Duration::from_millis(600);
                self.set_hover_stage(0);
            },
            Action::ToStage1=>{
                self.hover_blocked_until=Instant::now()+Duration::from_millis(600);
                self.set_hover_stage(1);
            },
            Action::CalendarToday=>{
                let now=Local::now().date_naive();
                self.year=now.year();
                self.month=now.month();
                self.day=now.day();
                self.load_holidays();
            },
            Action::CalendarLeaveToggle=>{
                self.settings.collective_leave=!self.settings.collective_leave;
            },
            Action::Tab(tab)=>{
                self.reminder_menu=None;
                if self.tab == tab && self.hover_stage >= 2 {
                    self.hover_blocked_until=Instant::now()+Duration::from_millis(600);
                    self.set_hover_stage(1);
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
                    self.settings_open=false;
                    let from=self.geometry();
                    self.morph=Some(ui::Morph::new(from,self.target_geometry()));
                    self.update_window_region();
                    self.start_morph_timer();
                    self.make_inputs();
                    self.invalidate();
                    unsafe{SetTimer(Some(self.hwnd),10,16,None);}
                }
            },
            Action::MoveTab(tab, delta)=>{
                self.settings.move_tab(&tab, delta);
                self.save();
                self.invalidate();
            },
            Action::SettingsCategory(cat)=>{self.settings_cat=cat.min(4);self.settings_popup=None;self.focused=Some(Action::SettingsCategory(self.settings_cat));},
            Action::SettingsMenu(key,x,y,w)=>self.open_settings_menu(key,x,y,w),
            Action::SettingsValue(key,value)=>{self.apply_settings_value(key,value);self.settings_popup=None;},
            Action::SettingsOpenTab(tab,index)=>{if !self.settings.enabled(&tab){return}let from=self.geometry();self.settings_open=false;self.settings_popup=None;self.reminder_menu=None;self.tab=tab;self.tab_fade=None;if self.tab=="Reminder"{self.select_reminder(index);}self.morph=Some(ui::Morph::new(from,self.target_geometry()));self.start_morph_timer();self.make_inputs();self.update_window_region();},
            Action::SetDefaultTab(tab)=>{if self.settings.enabled(&tab){self.settings.default_tab=tab;}},
            Action::Settings=>{self.settings_popup=None;self.reminder_menu=None;self.focused=None;self.settings_open=true;self.set_expanded(true);let from=self.geometry();self.morph=Some(ui::Morph::new(from,self.target_geometry()));self.update_window_region();self.start_morph_timer();self.make_inputs();self.invalidate();},
            Action::Back=>{self.settings_popup=None;self.focused=None;self.settings_open=false;let from=self.geometry();self.morph=Some(ui::Morph::new(from,self.target_geometry()));self.update_window_region();self.start_morph_timer();self.make_inputs();self.invalidate();},
            Action::Close=>self.set_expanded(false),Action::Exit=>unsafe{let _=DestroyWindow(self.hwnd);},
            Action::Bottom(value)=>{self.settings.bottom=value;self.layout();self.save();},Action::Minimal=>{self.settings.minimal=!self.settings.minimal;self.sync_visualizer();self.save();},Action::Gap(delta)=>{self.settings.edge_gap=(self.settings.edge_gap+delta).clamp(0,160);self.layout();self.save();},Action::Scale(delta)=>{self.settings.scale=(self.settings.scale+delta).clamp(0.8,1.35);self.layout();self.make_inputs();self.save();},Action::Opacity(delta)=>{self.settings.opacity=(self.settings.opacity as i32+delta).clamp(165,255)as u8;self.save();self.invalidate();},
            Action::ToggleModule(module)=>{let enabled=!self.settings.enabled(&module);if !self.settings.set_module_enabled(&module,enabled){self.notice("Sisakan setidaknya satu tab aktif.");return}if module=="Musik"{if enabled{self.start_media();}else{if let Some(mut m)=self.media.take(){m.stop();}self.track=MediaSnapshot::default();self.lyrics.clear();self.sync_visualizer();}}if module=="Reminder"&&!enabled{self.focus.reset();self.stretch_deadline=None;}if module=="Sehat"&&!enabled{self.breathing=None;}if !self.settings.enabled(&self.tab){self.tab=self.settings.active_tab_order().first().cloned().unwrap_or_default();self.make_inputs();}self.save();self.invalidate();},
            Action::Toggle(key)=>match key{
                "auto_break"=>self.settings.auto_break=!self.settings.auto_break,
                "focus_enabled"=>{self.settings.focus_enabled=!self.settings.focus_enabled;if !self.settings.focus_enabled{self.focus.pause();}},
                "stretch_after_focus"=>self.settings.stretch_after_focus=!self.settings.stretch_after_focus,
                "prayer_sound"=>self.settings.prayer_sound=!self.settings.prayer_sound,
                "prayer_enabled"=>{self.settings.prayer_enabled=!self.settings.prayer_enabled;self.refresh_prayers(false);},
                "message_preview"=>self.settings.message_preview=!self.settings.message_preview,
                "fullscreen"=>{self.settings.hide_fullscreen=!self.settings.hide_fullscreen;self.save();},
                "glow"=>{self.settings.glow=!self.settings.glow;self.save();},
                "visualizer"=>{self.settings.visualizer=!self.settings.visualizer;self.sync_visualizer();self.save();},
                "water_reminder"=>{self.settings.water_reminder=!self.settings.water_reminder;self.last_water=Instant::now();self.save();},
                "move_reminder"=>{self.settings.move_reminder=!self.settings.move_reminder;self.last_move=Instant::now();self.save();},
                "lyrics_below"=>self.settings.lyrics_below=!self.settings.lyrics_below,
                "save_schedule"=>{let schedule=self.input(100);let times:Vec<String>=schedule.split(',').map(str::trim).filter(|s|!s.is_empty()).map(str::to_owned).collect();if times.iter().any(|t|chrono::NaiveTime::parse_from_str(t,"%H:%M").is_err()){self.notice("Format jadwal harus HH:MM, misalnya 09:00,14:00.");return}self.settings.focus_schedule=times;let tab=self.input(101);if MODULES.contains(&tab.trim()){self.settings.default_tab=tab.trim().into();}self.notice("Jadwal dan tab default disimpan.");},
                
                _=>{}
            },
            Action::SeekPointer=>{},Action::LyricsFollow=>self.lyric_offset=0,
            Action::Play=>self.media_command(|m|m.toggle_playback()),Action::Next=>self.media_command(|m|m.next()),Action::Previous=>self.media_command(|m|m.previous()),Action::Seek(seconds)=>{if self.track.can_seek{self.media_command(|m|m.seek(seconds));}},Action::Lyric(index)=>{if self.track.can_seek{if let Some(line)=self.lyrics.get(index){let at=line.at;self.media_command(|m|m.seek(at));}}},
            Action::ReminderSelect(i)=>self.select_reminder(i),
            Action::ReminderStep(d)=>self.select_reminder((self.reminder_index as i32+d).clamp(0,3)as usize),
            Action::StretchLength(d)=>self.settings.stretch_seconds=(self.settings.stretch_seconds as i32+d).clamp(15,300)as u32,
            Action::StretchDone=>{self.stretch_deadline=None;self.last_move=Instant::now();self.notice("Peregangan tercatat. Sampai jeda berikutnya.");},
            Action::StretchStart=>{if self.stretch_deadline.is_some(){self.stretch_deadline=None;self.last_move=Instant::now();}else{self.stretch_deadline=Some(Instant::now()+Duration::from_secs(self.settings.stretch_seconds as u64));}},
            Action::PrayerRefresh=>self.refresh_prayers(true),
            Action::ReminderMenu(key)=>{self.open_reminder_menu(key);},
            Action::ReminderValue(key,value)=>{self.apply_reminder_value(key,value);self.reminder_menu=None;},
            Action::PrayerLead(d)=>self.settings.prayer_lead=(self.settings.prayer_lead as i32+d).clamp(0,30)as u32,

            Action::FocusStart=>self.focus.start(self.settings.focus_minutes,false),Action::FocusBreak=>self.focus.start(self.settings.break_minutes,true),Action::FocusPause=>{if self.focus.running(){self.focus.pause();}else{self.focus.resume();}},Action::FocusReset=>self.focus.reset(),Action::FocusLength(delta)=>{self.settings.focus_minutes=(self.settings.focus_minutes as i32+delta).clamp(1,180)as u32;self.save();},Action::BreakLength(delta)=>{self.settings.break_minutes=(self.settings.break_minutes as i32+delta).clamp(1,60)as u32;self.save();},
            Action::AddTodo=>{let text=self.input(10).trim().to_owned();if text.is_empty(){return}let due_text=self.input(11);let due=if due_text.trim().is_empty(){None}else{use chrono::TimeZone;match chrono::NaiveDateTime::parse_from_str(due_text.trim(),"%Y-%m-%d %H:%M").ok().and_then(|d|Local.from_local_datetime(&d).single()){Some(time)=>Some(time),None=>{self.notice("Tanggal pengingat harus YYYY-MM-DD HH:MM.");return}}};self.state.todos.push(Todo{id:Local::now().timestamp_millis()as u64,text,done:false,due,reminded:false});self.clear_input(10);self.clear_input(11);},Action::Todo(id)=>{if let Some(t)=self.state.todos.iter_mut().find(|t|t.id==id){t.done=!t.done;}},Action::DeleteTodo(id)=>self.state.todos.retain(|t|t.id!=id),Action::TodoPage(delta)=>self.todo_offset=(self.todo_offset as i32+delta).clamp(0,self.state.todos.len().saturating_sub(1)as i32)as usize,Action::SaveNote=>{self.state.notes=self.input(12);self.notice("Catatan disimpan.");},
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
                    toast.message=format!("Hebat! +{}ml tercatat 🎉",serving);
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
                        self.notice("Teks disalin ke clipboard! 📋");
                    }else{
                        self.notice("Gagal menyalin ke clipboard.");
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
            Action::ClearClips=>{
                self.state.clipboard_history.clear();
                self.selected_clip=0;
                self.clip_offset=0;
                self.save();
                self.notice("Riwayat clipboard dibersihkan.");
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
            Action::WeatherSave=>{match(self.input(31).trim().parse::<f64>(),self.input(32).trim().parse::<f64>()){(Ok(lat),Ok(lon))if lat.is_finite()&&lon.is_finite()&&(-90.0..=90.0).contains(&lat)&&(-180.0..=180.0).contains(&lon)=>{self.settings.latitude=lat;self.settings.longitude=lon;self.settings.weather_city=self.input(30);self.settings.weather_configured=true;self.refresh_weather();},_=>self.notice("Latitude/longitude tidak valid.")}},Action::WeatherRefresh=>self.refresh_weather(),
            Action::AiSave=>{let key=self.input(43);if !key.is_empty(){match crate::secrets::protect(&key){Ok(secret)=>self.settings.ai_key=secret,Err(e)=>{self.notice(e);return}}}self.settings.ai_endpoint=self.input(41).trim().into();self.settings.ai_model=self.input(42).trim().into();self.clear_input(43);self.notice("Pengaturan AI tersimpan.");},Action::AiSend=>self.send_ai(),Action::AiClear=>self.ai_messages.clear(),
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
        }
        for module in MODULES{if !self.settings.enabled(module){self.cancel_job(module);}}
        if !self.settings.enabled("Musik"){self.cancel_job("Artwork");self.cover=None;self.cover_key=0;self.lyrics_key.clear();self.plain_lyrics.clear();self.lyric_offset=0;}
        self.sync_foreground();self.sync_visualizer();if !transient{self.save();}self.clock_timer();self.invalidate();
    }
    fn media_command(&mut self,command:impl FnOnce(&MediaService)->Result<(),String>){if let Some(media)=&self.media{if let Err(e)=command(media){self.notice(e);}}else{self.notice("Buka pemutar musik terlebih dahulu.");}}
    fn refresh_weather(&mut self){if !self.settings.enabled("Cuaca")||!self.settings.weather_configured{self.notice("Simpan lokasi cuaca terlebih dahulu.");return}if self.jobs.contains_key("Cuaca"){return}let api=self.online.clone();let(lat,lon)=(self.settings.latitude,self.settings.longitude);self.job("Cuaca",move||Event::Weather(api.weather(lat,lon)));}
    fn refresh_agenda(&mut self){if !self.settings.enabled("Kalender"){return}let feed=match crate::secrets::unprotect(&self.settings.calendar_feed){Ok(feed)=>feed,Err(e)=>{self.notice(e);return}};if feed.is_empty(){self.cancel_job("Kalender");self.agenda.clear();self.notice("Feed kalender dilepas.");return}if self.jobs.contains_key("Kalender"){return}let api=self.online.clone();self.job("Kalender",move||Event::Agenda(api.agenda(&feed)));}
    fn send_ai(&mut self){if self.jobs.contains_key("AI")||self.fullscreen||!self.settings.enabled("AI"){return}let text=self.input(40).trim().to_owned();if text.is_empty(){return}let key=match crate::secrets::unprotect(&self.settings.ai_key){Ok(key)if!key.is_empty()=>key,_=>{self.notice("Simpan API key dan model terlebih dahulu.");return}};if self.settings.ai_model.trim().is_empty(){self.notice("Nama model belum diisi.");return}self.ai_messages.push(crate::online::ChatMessage{role:"user".into(),content:text});self.clear_input(40);let api=self.online.clone();let settings=self.settings.clone();let messages=self.ai_messages.clone();self.job("AI",move||Event::Ai(api.chat(&settings.ai_provider,&settings.ai_endpoint,&settings.ai_model,&key,&messages)));}
}

