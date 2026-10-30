use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use crate::privacy::model::PrivacySnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailboxError {
    StaleGeneration {
        snapshot_generation: u64,
        mailbox_generation: u64,
    },
    Closed,
}

impl std::fmt::Display for MailboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailboxError::StaleGeneration {
                snapshot_generation,
                mailbox_generation,
            } => write!(
                f,
                "Stale snapshot generation {snapshot_generation} < mailbox generation {mailbox_generation}"
            ),
            MailboxError::Closed => write!(f, "Mailbox closed"),
        }
    }
}

impl std::error::Error for MailboxError {}

struct MailboxInner {
    generation: u64,
    latest: Option<PrivacySnapshot>,
    wake_flag: bool,
    closed: bool,
}

pub struct MailboxSender {
    inner: Arc<(Mutex<MailboxInner>, Condvar)>,
}

impl Clone for MailboxSender {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

pub struct MailboxReceiver {
    inner: Arc<(Mutex<MailboxInner>, Condvar)>,
}

pub fn mailbox(initial_generation: u64) -> (MailboxSender, MailboxReceiver) {
    let inner = Arc::new((
        Mutex::new(MailboxInner {
            generation: initial_generation,
            latest: None,
            wake_flag: false,
            closed: false,
        }),
        Condvar::new(),
    ));

    (
        MailboxSender {
            inner: Arc::clone(&inner),
        },
        MailboxReceiver { inner },
    )
}

impl MailboxSender {
    /// Post a new snapshot. Rejects stale generations.
    /// Overwrites previous unconsumed snapshot (bounded, latest-only)
    /// and sets the coalesced wake flag.
    pub fn post(&self, snapshot: PrivacySnapshot) -> Result<(), MailboxError> {
        let (lock, cvar) = &*self.inner;
        let mut inner = lock.lock().unwrap();

        if inner.closed {
            return Err(MailboxError::Closed);
        }

        if snapshot.generation < inner.generation {
            return Err(MailboxError::StaleGeneration {
                snapshot_generation: snapshot.generation,
                mailbox_generation: inner.generation,
            });
        }

        if snapshot.generation > inner.generation {
            inner.generation = snapshot.generation;
        }

        inner.latest = Some(snapshot);
        inner.wake_flag = true;
        cvar.notify_all();
        Ok(())
    }

    pub fn current_generation(&self) -> u64 {
        let (lock, _) = &*self.inner;
        lock.lock().unwrap().generation
    }
}

impl MailboxReceiver {
    /// Non-blocking check for the latest snapshot.
    /// Returns Some(snapshot) and clears the wake flag if an update was available.
    /// Subsequent calls return None until another snapshot is posted.
    pub fn poll_latest(&self) -> Option<PrivacySnapshot> {
        let (lock, _) = &*self.inner;
        let mut inner = lock.lock().unwrap();
        if inner.wake_flag {
            inner.wake_flag = false;
            inner.latest.clone()
        } else {
            None
        }
    }

    /// Non-blocking check taking the latest snapshot out of the mailbox.
    pub fn take_latest(&self) -> Option<PrivacySnapshot> {
        let (lock, _) = &*self.inner;
        let mut inner = lock.lock().unwrap();
        if inner.wake_flag {
            inner.wake_flag = false;
            inner.latest.take()
        } else {
            None
        }
    }

    /// Read the latest snapshot without clearing the wake flag.
    pub fn peek_latest(&self) -> Option<PrivacySnapshot> {
        let (lock, _) = &*self.inner;
        let inner = lock.lock().unwrap();
        inner.latest.clone()
    }

    /// Check if a wake event is currently flagged.
    pub fn has_wake(&self) -> bool {
        let (lock, _) = &*self.inner;
        let inner = lock.lock().unwrap();
        inner.wake_flag
    }

    /// Set a new generation threshold. Any snapshot currently buffered with
    /// an older generation will be discarded, and wake flag cleared.
    pub fn set_generation(&self, generation: u64) {
        let (lock, _) = &*self.inner;
        let mut inner = lock.lock().unwrap();
        inner.generation = generation;
        if let Some(ref s) = inner.latest {
            if s.generation < generation {
                inner.latest = None;
                inner.wake_flag = false;
            }
        }
    }

    pub fn current_generation(&self) -> u64 {
        let (lock, _) = &*self.inner;
        lock.lock().unwrap().generation
    }

    /// Bounded wait for next snapshot.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<PrivacySnapshot> {
        let (lock, cvar) = &*self.inner;
        let mut inner = lock.lock().unwrap();
        if inner.wake_flag {
            inner.wake_flag = false;
            return inner.latest.clone();
        }

        let result = cvar
            .wait_timeout_while(inner, timeout, |inner| !inner.wake_flag && !inner.closed)
            .ok()?;

        let mut inner = result.0;
        if inner.wake_flag {
            inner.wake_flag = false;
            inner.latest.clone()
        } else {
            None
        }
    }
}

impl Drop for MailboxReceiver {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.inner;
        if let Ok(mut inner) = lock.lock() {
            inner.closed = true;
            cvar.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mailbox_coalesced_post_and_poll() {
        let (tx, rx) = mailbox(1);
        assert_eq!(rx.poll_latest(), None);

        let mut snap1 = PrivacySnapshot::new(1);
        snap1.revision = 1;
        assert!(tx.post(snap1.clone()).is_ok());

        let mut snap2 = PrivacySnapshot::new(1);
        snap2.revision = 2;
        assert!(tx.post(snap2.clone()).is_ok());

        // Receiver polls once: gets latest (snap2), clearing wake flag
        let received = rx.poll_latest();
        assert_eq!(received.map(|s| s.revision), Some(2));

        // Second poll without posts returns None
        assert_eq!(rx.poll_latest(), None);
    }

    #[test]
    fn test_stale_generation_rejection() {
        let (tx, rx) = mailbox(2);

        // Snapshot from generation 1 is stale
        let snap_stale = PrivacySnapshot::new(1);
        let err = tx.post(snap_stale).unwrap_err();
        assert_eq!(
            err,
            MailboxError::StaleGeneration {
                snapshot_generation: 1,
                mailbox_generation: 2,
            }
        );

        // Mailbox did not receive stale snapshot
        assert_eq!(rx.poll_latest(), None);

        // Snapshot from generation 2 is accepted
        let snap_valid = PrivacySnapshot::new(2);
        assert!(tx.post(snap_valid).is_ok());
        assert_eq!(rx.poll_latest().map(|s| s.generation), Some(2));
    }

    #[test]
    fn test_receiver_set_generation_invalidates_stale() {
        let (tx, rx) = mailbox(1);

        let snap1 = PrivacySnapshot::new(1);
        assert!(tx.post(snap1).is_ok());
        assert!(rx.has_wake());

        // Bump generation to 2 on receiver
        rx.set_generation(2);

        // The buffered snapshot was for generation 1, so it should be discarded
        assert!(!rx.has_wake());
        assert_eq!(rx.poll_latest(), None);

        // Now post with generation 2
        let snap2 = PrivacySnapshot::new(2);
        assert!(tx.post(snap2).is_ok());
        assert_eq!(rx.poll_latest().map(|s| s.generation), Some(2));
    }
}
