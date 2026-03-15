use rand::Rng;

/// Holds a per-session random seed used to generate deterministic,
/// per-origin fingerprint noise.  Created once at app startup and
/// stored in Tauri managed state.  Immutable after construction.
pub struct FingerprintShield {
    session_seed: [u8; 32],
}

impl FingerprintShield {
    /// Create a new shield with a cryptographically random session seed.
    pub fn new() -> Self {
        let seed = rand::thread_rng().gen::<[u8; 32]>();
        Self { session_seed: seed }
    }

    /// Return the full JavaScript injection script with the session seed
    /// prepended as a global constant.
    pub fn get_injection_script(&self) -> String {
        let seed_hex: String = self.session_seed.iter().map(|b| format!("{b:02x}")).collect();
        let shield_js = include_str!("../../resources/fingerprint_shield.js");
        format!("const __VOID_SESSION_SEED = '{seed_hex}';\n{shield_js}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_contains_seed_and_markers() {
        let shield = FingerprintShield::new();
        let script = shield.get_injection_script();
        assert!(script.contains("__VOID_SESSION_SEED"));
        assert!(script.contains("hardwareConcurrency"));
        assert!(script.contains("toDataURL"));
        assert!(!script.is_empty());
    }

    #[test]
    fn two_instances_have_different_seeds() {
        let a = FingerprintShield::new();
        let b = FingerprintShield::new();
        // Extremely unlikely to collide
        assert_ne!(a.session_seed, b.session_seed);
    }
}
