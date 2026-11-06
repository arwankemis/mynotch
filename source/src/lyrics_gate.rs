//! Live consent snapshots: every change invalidates all earlier requests, even OFF -> ON.
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
#[derive(Clone, Default)]
pub struct LyricsConsent(Arc<AtomicU64>);
#[derive(Clone)]
pub struct LyricsPermission { current: Arc<AtomicU64>, revision: u64 }
impl LyricsConsent {
    pub fn set(&self, online: bool, youtube: bool) {
        let previous = self.0.load(Ordering::Acquire);
        self.0.store(((previous >> 2).wrapping_add(1) << 2) | u64::from(online) | (u64::from(youtube) << 1), Ordering::Release);
    }
    pub fn snapshot(&self) -> LyricsPermission {
        LyricsPermission { current: self.0.clone(), revision: self.0.load(Ordering::Acquire) }
    }
}
impl LyricsPermission {
    pub fn check(&self) -> Result<(), String> {
        crate::net::check_cancel()?;
        if self.revision & 1 == 0 || self.current.load(Ordering::Acquire) != self.revision {
            return Err("Lyrics consent revoked or disabled".into());
        }
        Ok(())
    }
    pub fn youtube(&self) -> bool { self.check().is_ok() && self.revision & 2 != 0 }
    pub fn revision(&self) -> u64 { self.revision }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn defaults_legacy_persistence_and_preserved_subordinate() {
        let legacy:crate::core::Settings=serde_json::from_str(r#"{"romanize_lyrics":true}"#).unwrap();
        assert!(!legacy.lyrics_online&&!legacy.lyrics_youtube_fallback);
        let mut settings=crate::core::Settings::default();assert!(!settings.lyrics_online&&!settings.lyrics_youtube_fallback);
        settings.lyrics_online=true;settings.lyrics_youtube_fallback=true;
        let saved=serde_json::to_string(&settings).unwrap();let mut restored:crate::core::Settings=serde_json::from_str(&saved).unwrap();
        let consent=LyricsConsent::default();consent.set(restored.lyrics_online,restored.lyrics_youtube_fallback);let old=consent.snapshot();assert!(old.youtube());
        restored.lyrics_online=false;consent.set(restored.lyrics_online,restored.lyrics_youtube_fallback);
        assert!(restored.lyrics_youtube_fallback);assert!(!consent.snapshot().youtube());
        restored.lyrics_online=true;consent.set(restored.lyrics_online,restored.lyrics_youtube_fallback);
        assert!(consent.snapshot().youtube());assert!(old.check().is_err());
    }
}
