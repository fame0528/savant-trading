//! Wallet Private Key Security — secret-string newtype (FID-211, audit Finding 1.1).
//!
//! Wraps `Secret<String>` so wallet private keys:
//! - Cannot be accidentally logged via `log!` / `info!` / `warn!` / `error!`
//! - Are redacted in `Display` and `Debug` impls (show `Secret(***)`)
//! - Are zeroized on drop (memory is scrubbed when the value goes out of scope)
//!
//! # Anti-Pattern (forbidden)
//!
//! ```ignore
//! let key: String = std::env::var("WALLET_PRIVATE_KEY")?;
//! info!("got key: {}", key);  // ❌ Key leaks to logs
//! ```
//!
//! # Correct usage
//!
//! ```ignore
//! let key = WalletKey::from_env("WALLET_PRIVATE_KEY")?;
//! info!("got key: {}", key);  // ✅ Logs "WalletKey(***)"
//! let hex = key.expose_secret();  // Use expose_secret() at the actual signing site
//! ```

use secrecy::{ExposeSecret, SecretBox};
use std::fmt;
use std::str::FromStr;

/// A wallet private key that cannot be accidentally leaked.
///
/// Wraps `Secret<String>`. `expose_secret()` is the only way to access the
/// underlying value, and it should be called only at the point of actual
/// cryptographic use (signing). Never log or store the exposed value.
pub struct WalletKey(SecretBox<String>);

// SAFETY: WalletKey holds a SecretBox<String> which uses zeroize-on-drop.
// We intentionally implement Clone manually — duplicating a SecretBox creates
// a new independent buffer that's zeroized on drop. (SecretBox doesn't derive
// Clone by design.)
impl Clone for WalletKey {
    fn clone(&self) -> Self {
        Self(SecretBox::new(Box::new(self.0.expose_secret().clone())))
    }
}

impl WalletKey {
    /// Wrap a string as a `WalletKey`. The string is moved into the SecretBox
    /// container and zeroized on drop.
    pub fn new(s: String) -> Self {
        Self(SecretBox::new(Box::new(s)))
    }

    /// Read a wallet key from an environment variable.
    pub fn from_env(env_var_name: &str) -> Result<Self, String> {
        let key =
            std::env::var(env_var_name).map_err(|_| format!("env var {} not set", env_var_name))?;
        Ok(Self::new(key))
    }

    /// Parse a wallet key from a string slice (useful for tests + config).
    pub fn parse(s: &str) -> Result<Self, String> {
        Ok(Self::new(s.to_string()))
    }

    /// Expose the secret value. Use ONLY at the signing site. Never log, never
    /// store the returned reference beyond the scope of one function call.
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

impl fmt::Debug for WalletKey {
    /// Always redacts. `Secret<T>` already redacts; we wrap it for clarity.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WalletKey(***)")
    }
}

impl fmt::Display for WalletKey {
    /// Always redacts. Prevents accidental format-string leaks.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WalletKey(***)")
    }
}

impl FromStr for WalletKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts() {
        let key = WalletKey::parse("0xdeadbeef00000000").unwrap();
        let debug = format!("{:?}", key);
        assert!(
            !debug.contains("deadbeef"),
            "Debug leaked secret: {}",
            debug
        );
        assert!(
            debug.contains("***"),
            "Debug missing redaction marker: {}",
            debug
        );
    }

    #[test]
    fn display_redacts() {
        let key = WalletKey::parse("0xdeadbeef00000000").unwrap();
        let display = format!("{}", key);
        assert!(
            !display.contains("deadbeef"),
            "Display leaked secret: {}",
            display
        );
        assert!(
            display.contains("***"),
            "Display missing redaction marker: {}",
            display
        );
    }

    #[test]
    fn expose_secret_returns_value() {
        let key = WalletKey::parse("0xdeadbeef00000000").unwrap();
        assert_eq!(key.expose_secret(), "0xdeadbeef00000000");
    }

    #[test]
    fn clone_exposes_same_value_but_redacts() {
        let key = WalletKey::parse("0xdeadbeef00000000").unwrap();
        let cloned = key.clone();
        assert_eq!(cloned.expose_secret(), key.expose_secret());
        // Both Debug impls redact
        assert!(!format!("{:?}", key).contains("deadbeef"));
        assert!(!format!("{:?}", cloned).contains("deadbeef"));
    }

    #[test]
    fn from_env_works() {
        std::env::set_var("TEST_WALLET_KEY_FID211", "0xtestkey12345");
        let key = WalletKey::from_env("TEST_WALLET_KEY_FID211").unwrap();
        assert_eq!(key.expose_secret(), "0xtestkey12345");
        std::env::remove_var("TEST_WALLET_KEY_FID211");
    }

    #[test]
    fn from_env_errors_on_missing() {
        std::env::remove_var("NONEXISTENT_WALLET_KEY_FID211");
        let result = WalletKey::from_env("NONEXISTENT_WALLET_KEY_FID211");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("NONEXISTENT_WALLET_KEY_FID211"));
    }

    /// Regression test for the actual bug: a panic message that contains a
    /// formatted WalletKey must NOT include the secret value.
    #[test]
    fn panic_message_redacts() {
        let key = WalletKey::parse("0xdeadbeef00000000").unwrap();
        let result = std::panic::catch_unwind(|| {
            panic!("intentional panic with key: {:?}", key);
        });
        let err = result.unwrap_err();
        let panic_msg = err
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| err.downcast_ref::<&str>().copied())
            .unwrap_or("");
        assert!(
            !panic_msg.contains("deadbeef"),
            "Panic leaked secret: {}",
            panic_msg
        );
    }
}
