//! Local activity model. The approved presenter will consume one immutable snapshot.
//! This module performs no HWND, audio, network, or timer operations.
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use crate::core::{Drink, State};

pub fn initial_sequence() -> u64 { 1 }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReminderKind { Focus, Stretch, Prayer, Todo, Water, Move, Eye }
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActivityId {
    Focus(u64), Stretch(u64),
    Prayer { city: String, date: String, slot: u8, time: String, lead: u32, latch: String },
    TodoDue { todo_id: u64, due_ms: i64 },
    Water(u64), Move(u64), Eye(u64),
    Media(u64), Hud(u64), Feedback(u64), Preview(u64), Clock,
}
impl ActivityId {
    pub fn kind(&self) -> Option<ReminderKind> { Some(match self {
        Self::Focus(_) => ReminderKind::Focus, Self::Stretch(_) => ReminderKind::Stretch,
        Self::Prayer { .. } => ReminderKind::Prayer, Self::TodoDue { .. } => ReminderKind::Todo,
        Self::Water(_) => ReminderKind::Water, Self::Move(_) => ReminderKind::Move,
        Self::Eye(_) => ReminderKind::Eye, _ => return None,
    }) }
    fn sequence(&self) -> Option<u64> { match self {
        Self::Focus(n)|Self::Stretch(n)|Self::Water(n)|Self::Move(n)|Self::Eye(n) => Some(*n), _ => None,
    } }
    fn recurring(&self) -> bool { matches!(self, Self::Water(_)|Self::Move(_)|Self::Eye(_)) }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityTarget { AudioSettings, Reminder(u8), Todo(u64), Music, Calendar, Unavailable }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingReminder {
    pub id: ActivityId, pub kind: ReminderKind, pub target: ActivityTarget,
    pub due_at_unix_ms: i64, pub remaining_visible_ms: u32,
    #[serde(default)] pub presented_once: bool,
    #[serde(default = "one")] pub coalesced_count: u32,
    #[serde(default)] pub silent: bool,
    #[serde(default)] pub deferred_count: u32,
}
fn one() -> u32 { 1 }
impl PendingReminder {
    pub fn new(id: ActivityId, due_ms: i64) -> Option<Self> {
        let kind = id.kind()?;
        let target = match &id {
            ActivityId::TodoDue { todo_id, .. } => ActivityTarget::Todo(*todo_id),
            _ => ActivityTarget::Reminder(match kind { ReminderKind::Focus => 0, ReminderKind::Water => 1, ReminderKind::Prayer => 3, _ => 2 }),
        };
        Some(Self { id, kind, target, due_at_unix_ms: due_ms,
            remaining_visible_ms: if matches!(kind, ReminderKind::Water|ReminderKind::Prayer) { 12_000 } else { 8_000 },
            presented_once: false, coalesced_count: 1, silent: false, deferred_count: 0 })
    }
}

impl State {
    pub fn normalize_activities(&mut self) {
        let highest = self.pending_local_reminders.iter().map(|p| &p.id)
            .chain(self.local_reminder_latches.iter()).filter_map(ActivityId::sequence).chain(self.local_recurring_high_water.values().copied()).max().unwrap_or(0);
        self.next_activity_sequence = self.next_activity_sequence.max(1).max(highest.saturating_add(1));
        let mut seen = BTreeMap::<ActivityId, PendingReminder>::new();
        for mut p in std::mem::take(&mut self.pending_local_reminders) {
            if p.id.kind() != Some(p.kind) || p.remaining_visible_ms == 0 { continue; }
            p.remaining_visible_ms = p.remaining_visible_ms.min(12_000);
            p.coalesced_count = p.coalesced_count.max(1);
            // Missing/ambiguous sources may be presented generically, never routed to a neighbour.
            if let Some(expected)=PendingReminder::new(p.id.clone(),p.due_at_unix_ms){
                if p.target!=expected.target {p.target=ActivityTarget::Unavailable;}
            }
            if let ActivityId::TodoDue { todo_id, .. } = p.id {
                if self.todos.iter().filter(|t| t.id == todo_id).count() != 1 { p.target = ActivityTarget::Unavailable; }
                else if p.target != ActivityTarget::Todo(todo_id) { p.target = ActivityTarget::Unavailable; }
            }
            seen.entry(p.id.clone()).and_modify(|old| {
                old.presented_once |= p.presented_once;
                old.silent |= p.silent; old.remaining_visible_ms = old.remaining_visible_ms.min(p.remaining_visible_ms);
            }).or_insert(p);
        }
        self.pending_local_reminders = seen.into_values().collect();
    }
    pub fn allocate_activity_sequence(&mut self) -> Result<u64, &'static str> {
        self.normalize_activities();
        let n = self.next_activity_sequence;
        self.next_activity_sequence = n.checked_add(1).ok_or("Identitas aktivitas habis")?;
        Ok(n)
    }
    pub fn enqueue_local_reminder(&mut self, p: PendingReminder) -> Result<bool, &'static str> {
        if p.id.kind() != Some(p.kind) || p.remaining_visible_ms == 0 { return Err("Pengingat tidak valid"); }
        if self.local_reminder_latches.contains(&p.id) || self.pending_local_reminders.iter().any(|old| old.id == p.id) { return Ok(false); }
        if p.id.recurring() {
            let sequence = p.id.sequence().unwrap();
            if self.local_recurring_high_water.get(&p.kind).is_some_and(|n| sequence <= *n) { return Ok(false); }
            if let Some(old) = self.pending_local_reminders.iter_mut().find(|old| old.kind == p.kind) {
                let count = old.coalesced_count.checked_add(1).ok_or("Jumlah pengingat habis")?;
                old.coalesced_count = count; self.local_recurring_high_water.insert(p.kind, sequence); return Ok(true);
            }
            self.local_recurring_high_water.insert(p.kind, sequence);
        }
        self.pending_local_reminders.push(p); Ok(true)
    }
    fn latch_activity(&mut self, id: &ActivityId) {
        self.local_reminder_latches.insert(id.clone());
        match id {
            ActivityId::TodoDue { todo_id, due_ms } => {
                if self.todos.iter().filter(|t| t.id == *todo_id).count() == 1 {
                    if let Some(t) = self.todos.iter_mut().find(|t| t.id == *todo_id) {
                        if t.due.as_ref().is_some_and(|due| due.timestamp_millis() == *due_ms) { t.reminded = true; }
                    }
                }
            }
            ActivityId::Prayer { latch, .. } => if !self.prayer_delivered.contains(latch) { self.prayer_delivered.push(latch.clone()); },
            _ => {},
        }
    }
    pub fn dismiss_local_reminder(&mut self, id: &ActivityId) -> bool {
        if !self.pending_local_reminders.iter().any(|p| &p.id == id) { return false; }
        self.latch_activity(id); self.pending_local_reminders.retain(|p| &p.id != id); true
    }
    /// Adapter for the existing Todo source. Disabled modules retain existing pending data.
    pub fn collect_due_todos(&mut self, now_ms: i64, enabled: bool) {
        self.pending_local_reminders.retain(|p| match &p.id {
            ActivityId::TodoDue { todo_id, due_ms } => self.todos.iter().any(|t| t.id == *todo_id && !t.done && t.due.as_ref().is_some_and(|d| d.timestamp_millis() == *due_ms)),
            _ => true,
        });
        if !enabled { return; }
        let due: Vec<_> = self.todos.iter().filter(|t| !t.done && !t.reminded)
            .filter_map(|t| t.due.as_ref().filter(|d| d.timestamp_millis() <= now_ms).map(|d| (ActivityId::TodoDue { todo_id: t.id, due_ms: d.timestamp_millis() },d.timestamp_millis()))).collect();
        for (id,due_ms) in due { if let Some(p) = PendingReminder::new(id, due_ms) { let _ = self.enqueue_local_reminder(p); } }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PresentationPolicy { pub visible: bool, pub quiet: bool, pub reminders: bool, pub media: bool, pub reduced_motion: bool }
impl Default for PresentationPolicy { fn default() -> Self { Self { visible: true, quiet: false, reminders: true, media: true, reduced_motion: false } } }
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub id: ActivityId, pub priority: u8, pub created_ms: i64, pub expires_ms: Option<i64>,
    pub target: Option<ActivityTarget>, pub text: String, pub progress: Option<f32>,
}
impl Candidate {
    pub fn bounded(mut self) -> Self { self.progress = self.progress.filter(|v| v.is_finite()).map(|v| v.clamp(0., 1.)); self }
}
#[derive(Clone, Debug, Default)]
pub struct ActivityInputs { pub candidates: Vec<Candidate> }
impl ActivityInputs {
    pub fn media(&mut self, track: &crate::media::MediaSnapshot, now_ms: i64) {
        if track.playing && !track.session_id.is_empty() {
            self.candidates.push(Candidate { id: ActivityId::Media(track.selection_revision), priority: 4, created_ms: now_ms,
                expires_ms: None, target: Some(ActivityTarget::Music), text: track.title.clone(), progress: None });
        }
    }
    pub fn countdown(&mut self, id: ActivityId, started_ms: i64, remaining_ms: u64, total_ms: u64, running: bool) {
        if running && remaining_ms > 0 {
            self.candidates.push(Candidate { target: Some(ActivityTarget::Reminder(if matches!(id,ActivityId::Focus(_)) {0} else {2})),
                id, priority: 3, created_ms: started_ms, expires_ms: None, text: format!("{:02}:{:02}",remaining_ms/60_000,remaining_ms/1_000%60),
                progress: Some(if total_ms == 0 {0.} else {1.-remaining_ms as f32/total_ms as f32}) });
        }
    }
    pub fn hud(&mut self, generation: u64, now_ms: i64, percent: u8, muted: bool) {
        self.candidates.push(Candidate { id: ActivityId::Hud(generation), priority: 0, created_ms: now_ms, expires_ms: Some(now_ms.saturating_add(1_800)),
            target: Some(ActivityTarget::AudioSettings), text: if muted {"Mute".into()} else {format!("{}%",percent.min(100))}, progress: None });
    }
}

/// Runtime generation adapter for FocusClock/stretch. Pause/resume retains this
/// identity; only a new start allocates another run. It never resets the source.
#[derive(Default)]
pub struct RunTracker { pub id: Option<ActivityId>, pub started_ms: i64, completed: bool }
impl RunTracker {
    pub fn start(&mut self,state:&mut State,kind:ReminderKind,now_ms:i64)->Result<ActivityId,&'static str>{
        if !matches!(kind,ReminderKind::Focus|ReminderKind::Stretch){return Err("Jenis countdown tidak valid");}
        let sequence=state.allocate_activity_sequence()?;
        let id=if kind==ReminderKind::Focus{ActivityId::Focus(sequence)}else{ActivityId::Stretch(sequence)};
        self.id=Some(id.clone());self.started_ms=now_ms;self.completed=false;Ok(id)
    }
    pub fn complete(&mut self,state:&mut State,id:&ActivityId,now_ms:i64)->Result<bool,&'static str>{
        if self.id.as_ref()!=Some(id)||self.completed{return Ok(false);}
        let added=state.enqueue_local_reminder(PendingReminder::new(id.clone(),now_ms).ok_or("Run tidak valid")?)?;
        self.completed=true;Ok(added)
    }
    pub fn focus_input(&self,inputs:&mut ActivityInputs,clock:&crate::core::FocusClock){
        if let Some(id)=&self.id{inputs.countdown(id.clone(),self.started_ms,clock.remaining().as_millis().min(u64::MAX as u128) as u64,clock.total().as_millis().min(u64::MAX as u128) as u64,clock.running());}
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ActivitySnapshot { pub primary: Candidate, pub other_count: usize, pub revision: u64, pub animate: bool }
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PresentationEffects { pub first_delivery: Option<ActivityId>, pub sound: bool }
#[derive(Default)]
pub struct ActivityRuntime { last_ms: Option<i64>, last_visible: bool, pub snapshot: Option<ActivitySnapshot> }
fn select(state: &State, inputs: &ActivityInputs, now_ms: i64, policy: PresentationPolicy) -> (Candidate, usize) {
    let mut candidates: BTreeMap<ActivityId, Candidate> = BTreeMap::new();
    for c in &inputs.candidates {
        if c.id == ActivityId::Clock || c.expires_ms.is_some_and(|end| now_ms >= end) { continue; }
        if matches!(c.id, ActivityId::Media(_)) && !policy.media { continue; }
        if matches!(c.target,Some(ActivityTarget::Reminder(_)|ActivityTarget::Todo(_))) && !policy.reminders {continue;}
        if policy.quiet && matches!(c.id, ActivityId::Feedback(_)|ActivityId::Preview(_)) { continue; }
        let mut c = c.clone().bounded();
        if policy.quiet && !matches!(c.id, ActivityId::Hud(_)|ActivityId::Focus(_)|ActivityId::Stretch(_)) { c.text = "Aktivitas".into(); }
        candidates.entry(c.id.clone()).and_modify(|old| if (c.priority,c.created_ms) < (old.priority,old.created_ms) { *old=c.clone(); }).or_insert(c);
    }
    if policy.reminders && !policy.quiet {
        for p in &state.pending_local_reminders {
            if p.remaining_visible_ms == 0 { continue; }
            candidates.insert(p.id.clone(),Candidate { id:p.id.clone(),priority:2,created_ms:p.due_at_unix_ms,expires_ms:None,
                target:Some(p.target.clone()),text:format!("{:?}",p.kind),progress:None });
        }
    }
    let primary=candidates.values().min_by(|a,b| (a.priority,a.created_ms,a.id.kind(),&a.id).cmp(&(b.priority,b.created_ms,b.id.kind(),&b.id)))
        .cloned()
        .unwrap_or(Candidate {id:ActivityId::Clock,priority:5,created_ms:0,expires_ms:None,target:Some(ActivityTarget::Calendar),text:String::new(),progress:None});
    let other_count=candidates.values().filter(|c| c.priority <= 2 && c.id != primary.id).count();
    (primary,other_count)
}
impl ActivityRuntime {
    pub fn pause_presentation(&mut self){self.last_visible=false;}
    pub fn is_current(&self,state:&State,inputs:&ActivityInputs,now_ms:i64,policy:PresentationPolicy,id:&ActivityId,revision:u64)->bool{
        policy.visible&&self.snapshot.as_ref().is_some_and(|s|s.revision==revision&&&s.primary.id==id)&&&select(state,inputs,now_ms,policy).0.id==id
    }
    pub fn reconcile(&mut self,state:&mut State,inputs:&ActivityInputs,now_ms:i64,policy:PresentationPolicy)->PresentationEffects {
        if policy.visible && self.last_visible {
            let elapsed=now_ms.saturating_sub(self.last_ms.unwrap_or(now_ms)).max(0).min(u32::MAX as i64) as u32;
            if let Some(old)=&self.snapshot {
                if let Some(p)=state.pending_local_reminders.iter_mut().find(|p|p.id==old.primary.id) {
                    p.remaining_visible_ms=p.remaining_visible_ms.saturating_sub(elapsed);
                }
            }
        }
        let expired:Vec<_>=state.pending_local_reminders.iter().filter(|p|p.remaining_visible_ms==0).map(|p|p.id.clone()).collect();
        for id in expired { state.dismiss_local_reminder(&id); }
        if policy.quiet { for p in &mut state.pending_local_reminders {p.silent=true;} }
        let (primary,other_count)=select(state,inputs,now_ms,policy);
        let revision=self.snapshot.as_ref().map(|s|s.revision).unwrap_or(0);
        let animate=policy.visible&&!policy.reduced_motion;
        let changed=self.snapshot.as_ref().is_none_or(|s|s.primary.id!=primary.id||s.primary.target!=primary.target||s.other_count!=other_count||s.animate!=animate);
        self.snapshot=Some(ActivitySnapshot {primary:primary.clone(),other_count,revision:if changed {revision.saturating_add(1)}else{revision},animate});
        self.last_ms=Some(now_ms);self.last_visible=policy.visible&&!policy.quiet;
        let mut effects=PresentationEffects::default();
        if policy.visible && policy.reminders && !policy.quiet {
            if let Some(p)=state.pending_local_reminders.iter_mut().find(|p|p.id==primary.id) {
                if !p.presented_once {p.presented_once=true;effects.first_delivery=Some(p.id.clone());effects.sound=!p.silent;}
            }
            if let Some(id)=&effects.first_delivery {state.latch_activity(id);}
        }
        effects
    }
    pub fn open(&self,state:&State,inputs:&ActivityInputs,now_ms:i64,policy:PresentationPolicy,id:&ActivityId,revision:u64)->Option<ActivityTarget> {
        let s=self.snapshot.as_ref()?;
        if !policy.visible || s.revision!=revision || &s.primary.id!=id {return None;}
        let (current,_)=select(state,inputs,now_ms,policy);
        if &current.id!=id || current.target==Some(ActivityTarget::Unavailable) {return None;}
        if let Some(ActivityTarget::Todo(todo_id))=&current.target{
            if state.todos.iter().filter(|t|t.id==*todo_id&&!t.done).count()!=1{return None;}
            if let ActivityId::TodoDue{due_ms,..}=id{
                if !state.todos.iter().any(|t|t.id==*todo_id&&t.due.as_ref().is_some_and(|d|d.timestamp_millis()==*due_ms)){return None;}
            }
        }
        current.target
    }
    pub fn confirm_water(&self,state:&mut State,id:&ActivityId,serving:u32,date:chrono::NaiveDate)->bool {
        if !matches!(id,ActivityId::Water(_)) || self.snapshot.as_ref().is_none_or(|s|&s.primary.id!=id) || !state.pending_local_reminders.iter().any(|p|&p.id==id) {return false;}
        state.water.push(Drink {date,ml:serving});state.dismiss_local_reminder(id);true
    }
}

/// Flush/persistence adapters return generic status; no source text is logged.
pub fn save_pending(state:&State,store:&mut crate::core::Store)->Result<(),&'static str> {
    store.save("state.json",state).map_err(|_|"Aktivitas belum tersimpan")
}

#[cfg(test)]
#[path = "activity_tests.rs"]
mod tests;
