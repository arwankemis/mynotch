pub mod archive;
pub mod crypto;
pub mod download;
pub mod helper;
pub mod model;
pub mod transaction;
pub mod ui;

pub use model::{
    arch_matches, current_profile, select_candidate, BuildIdentity, PrereleaseInfo,
    VersionManifest,
};

pub use ui::{AvailableUpdate, PendingUpdate, UpdateStatus};

#[cfg(test)]
mod rehearsal_tests;
