use serde::{Deserialize, Serialize};

pub const CURRENT_ARCH: &str = "x86_64";
pub const CURRENT_OS: &str = "windows";
pub const VERSION_MANIFEST_RAW: &str = include_str!("../../../release/version.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrereleaseInfo {
    pub label: String,
    pub sequence: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionManifest {
    pub product: String,
    pub display_version: String,
    pub version_components: Vec<u32>,
    pub prerelease: Option<PrereleaseInfo>,
    pub release_sequence: u64,
    pub channel: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildIdentity {
    pub product: String,
    pub version: String,
    pub version_components: Vec<u32>,
    pub release_sequence: u64,
    pub profile: String,
    pub arch: String,
    pub os: String,
    pub channel: String,
}

pub fn current_profile() -> &'static str {
    if cfg!(feature = "personal") {
        "personal"
    } else {
        "public"
    }
}

fn normalize_arch(arch: &str) -> &str {
    let trimmed = arch.trim();
    if trimmed.eq_ignore_ascii_case("x64") || trimmed.eq_ignore_ascii_case("x86_64") {
        "x86_64"
    } else if trimmed.eq_ignore_ascii_case("arm64") || trimmed.eq_ignore_ascii_case("aarch64") {
        "arm64"
    } else {
        trimmed
    }
}

pub fn arch_matches(expected: &str, candidate: &str) -> bool {
    normalize_arch(expected).eq_ignore_ascii_case(normalize_arch(candidate))
}

impl BuildIdentity {
    pub fn current() -> Self {
        let manifest: VersionManifest = serde_json::from_str(VERSION_MANIFEST_RAW)
            .expect("release/version.json must be valid JSON");
        Self {
            product: manifest.product,
            version: manifest.display_version,
            version_components: manifest.version_components,
            release_sequence: manifest.release_sequence,
            profile: current_profile().to_string(),
            arch: CURRENT_ARCH.to_string(),
            os: CURRENT_OS.to_string(),
            channel: manifest.channel,
        }
    }

    pub fn new(profile: &str, release_sequence: u64) -> Self {
        Self {
            product: "mynotch".to_string(),
            version: String::new(),
            version_components: Vec::new(),
            release_sequence,
            profile: profile.to_string(),
            arch: CURRENT_ARCH.to_string(),
            os: CURRENT_OS.to_string(),
            channel: "preview".to_string(),
        }
    }

    pub fn with_arch(mut self, arch: impl Into<String>) -> Self {
        self.arch = arch.into();
        self
    }

    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = channel.into();
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }
}

pub fn select_candidate(
    current: &BuildIdentity,
    candidate_profile: &str,
    candidate_arch: &str,
    candidate_channel: &str,
    candidate_sequence: u64,
) -> bool {
    // Exact profile matching: Public only accepts Public, Personal only accepts Personal
    if !current.profile.eq_ignore_ascii_case(candidate_profile.trim()) {
        return false;
    }

    // Architecture enforcement
    if !arch_matches(&current.arch, candidate_arch) {
        return false;
    }

    // Channel enforcement
    if !current.channel.eq_ignore_ascii_case(candidate_channel.trim()) {
        return false;
    }

    // Monotonic sequence ordering: rejects older or equal sequence numbers
    if candidate_sequence <= current.release_sequence {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_profile_matching() {
        let public_current = BuildIdentity::new("public", 100);
        let personal_current = BuildIdentity::new("personal", 100);

        // Public rejects Personal candidate
        assert!(!select_candidate(&public_current, "personal", "x86_64", "preview", 101));
        assert!(!select_candidate(&public_current, "Personal", "x86_64", "preview", 101));

        // Public accepts Public candidate
        assert!(select_candidate(&public_current, "public", "x86_64", "preview", 101));
        assert!(select_candidate(&public_current, "Public", "x86_64", "preview", 101));

        // Personal rejects Public candidate
        assert!(!select_candidate(&personal_current, "public", "x86_64", "preview", 101));
        assert!(!select_candidate(&personal_current, "Public", "x86_64", "preview", 101));

        // Personal accepts Personal candidate
        assert!(select_candidate(&personal_current, "personal", "x86_64", "preview", 101));
        assert!(select_candidate(&personal_current, "Personal", "x86_64", "preview", 101));

        // Unknown profile rejected
        assert!(!select_candidate(&public_current, "enterprise", "x86_64", "preview", 101));
        assert!(!select_candidate(&personal_current, "enterprise", "x86_64", "preview", 101));
    }

    #[test]
    fn test_monotonic_sequence_ordering() {
        let current = BuildIdentity::new("public", 10401);

        // Older sequence rejected
        assert!(!select_candidate(&current, "public", "x86_64", "preview", 10400));
        assert!(!select_candidate(&current, "public", "x86_64", "preview", 100));
        assert!(!select_candidate(&current, "public", "x86_64", "preview", 0));

        // Equal sequence rejected
        assert!(!select_candidate(&current, "public", "x86_64", "preview", 10401));

        // Strictly newer sequence accepted
        assert!(select_candidate(&current, "public", "x86_64", "preview", 10402));
        assert!(select_candidate(&current, "public", "x86_64", "preview", 20000));
    }

    #[test]
    fn test_architecture_enforcement() {
        let current = BuildIdentity::new("public", 100).with_arch("x86_64");

        // Exact match
        assert!(select_candidate(&current, "public", "x86_64", "preview", 101));
        // x64 alias
        assert!(select_candidate(&current, "public", "x64", "preview", 101));
        assert!(select_candidate(&current, "public", "X64", "preview", 101));

        // ARM64, x86, or invalid rejected
        assert!(!select_candidate(&current, "public", "arm64", "preview", 101));
        assert!(!select_candidate(&current, "public", "aarch64", "preview", 101));
        assert!(!select_candidate(&current, "public", "x86", "preview", 101));
        assert!(!select_candidate(&current, "public", "i686", "preview", 101));
        assert!(!select_candidate(&current, "public", "riscv64", "preview", 101));
    }

    #[test]
    fn test_channel_enforcement() {
        let preview_current = BuildIdentity::new("public", 100).with_channel("preview");
        let stable_current = BuildIdentity::new("public", 100).with_channel("stable");

        // Preview accepts preview, rejects stable
        assert!(select_candidate(&preview_current, "public", "x86_64", "preview", 101));
        assert!(!select_candidate(&preview_current, "public", "x86_64", "stable", 101));

        // Stable accepts stable, rejects preview
        assert!(select_candidate(&stable_current, "public", "x86_64", "stable", 101));
        assert!(!select_candidate(&stable_current, "public", "x86_64", "preview", 101));

        // Unknown channel rejected
        assert!(!select_candidate(&preview_current, "public", "x86_64", "nightly", 101));
    }

    #[test]
    fn test_current_build_identity_from_manifest() {
        let identity = BuildIdentity::current();
        assert_eq!(identity.product, "mynotch");
        assert_eq!(identity.version, "0.0.1.4-alpha.3");
        assert_eq!(identity.version_components, vec![0, 0, 1, 4]);
        assert_eq!(identity.release_sequence, 10403);
        assert_eq!(identity.channel, "preview");
        assert_eq!(identity.arch, "x86_64");
        assert_eq!(identity.os, "windows");

        #[cfg(feature = "personal")]
        assert_eq!(identity.profile, "personal");

        #[cfg(not(feature = "personal"))]
        assert_eq!(identity.profile, "public");
    }
}
