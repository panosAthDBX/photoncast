//! Encryption manager for clipboard data.
//!
//! Uses AES-256-GCM for encryption with a machine-derived key via argon2.
//! The key is deterministic per machine, ensuring encrypted data persists
//! across application restarts.
//!
//! Salt is generated randomly on first use and stored securely in macOS Keychain.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use rand::RngCore;
use std::sync::Arc;

use crate::error::{ClipboardError, Result};

/// Nonce size for AES-256-GCM (96 bits = 12 bytes).
const NONCE_SIZE: usize = 12;

/// Salt size for key derivation (256 bits = 32 bytes for strong security).
const SALT_SIZE: usize = 32;

/// Keychain service name for storing the salt.
const KEYCHAIN_SERVICE: &str = "com.photoncast.clipboard";

/// Keychain account name for the salt.
const KEYCHAIN_ACCOUNT: &str = "encryption-salt";

/// Encryption manager for clipboard content.
///
/// Uses AES-256-GCM with a machine-derived key for secure storage.
/// The key is derived from machine-specific data using argon2, ensuring
/// the same key is generated on each application start.
#[derive(Clone)]
pub struct EncryptionManager {
    cipher: Arc<Aes256Gcm>,
}

impl std::fmt::Debug for EncryptionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionManager")
            .field("cipher", &"<AES-256-GCM>")
            .finish()
    }
}

impl EncryptionManager {
    /// Creates a new encryption manager with a machine-derived key.
    ///
    /// # Errors
    ///
    /// Returns an error if key derivation fails.
    pub fn new() -> Result<Self> {
        let machine_id = get_machine_id()?;
        Self::from_machine_id(&machine_id)
    }

    /// Creates an encryption manager from a specific machine ID.
    ///
    /// Useful for testing or when you want to use a custom identifier.
    pub fn from_machine_id(machine_id: &str) -> Result<Self> {
        let key = derive_key(machine_id.as_bytes())?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| ClipboardError::encryption(format!("failed to create cipher: {}", e)))?;

        Ok(Self {
            cipher: Arc::new(cipher),
        })
    }

    /// Creates an encryption manager for testing with a fixed key.
    ///
    /// # Warning
    ///
    /// This should only be used for testing. The key is not machine-specific.
    #[cfg(test)]
    pub fn for_testing() -> Result<Self> {
        Self::from_machine_id("test-machine-id-for-testing-only")
    }

    /// Encrypts plaintext data.
    ///
    /// Returns a byte vector containing the nonce prepended to the ciphertext.
    ///
    /// # Errors
    ///
    /// Returns an error if encryption fails.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        // Generate random nonce
        let nonce = generate_nonce();
        let nonce_array = Nonce::from_slice(&nonce);

        // Encrypt
        let ciphertext = self
            .cipher
            .encrypt(nonce_array, plaintext)
            .map_err(|e| ClipboardError::encryption(format!("encryption failed: {}", e)))?;

        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypts ciphertext data.
    ///
    /// Expects the input to have the nonce prepended (as produced by `encrypt`).
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails or data is malformed.
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < NONCE_SIZE {
            return Err(ClipboardError::encryption(
                "ciphertext too short: missing nonce",
            ));
        }

        // Split nonce and ciphertext
        let (nonce, data) = ciphertext.split_at(NONCE_SIZE);
        let nonce_array = Nonce::from_slice(nonce);

        // Decrypt
        let plaintext = self
            .cipher
            .decrypt(nonce_array, data)
            .map_err(|e| ClipboardError::encryption(format!("decryption failed: {}", e)))?;

        Ok(plaintext)
    }

    /// Encrypts a string.
    pub fn encrypt_string(&self, plaintext: &str) -> Result<Vec<u8>> {
        self.encrypt(plaintext.as_bytes())
    }

    /// Decrypts to a string.
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails or the result is not valid UTF-8.
    pub fn decrypt_string(&self, ciphertext: &[u8]) -> Result<String> {
        let plaintext = self.decrypt(ciphertext)?;
        String::from_utf8(plaintext).map_err(|e| {
            ClipboardError::encryption(format!("decrypted data is not valid UTF-8: {}", e))
        })
    }
}

/// Derives a 256-bit key from input data using argon2 with a secure random salt.
/// 
/// The salt is retrieved from (or generated and stored in) the macOS Keychain.
#[cfg(not(test))]
fn derive_key(input: &[u8]) -> Result<[u8; 32]> {
    let salt = get_or_create_salt()?;
    let mut key = [0u8; 32];

    // Use argon2id (recommended for password hashing and key derivation)
    let argon2 = Argon2::default();

    argon2
        .hash_password_into(input, &salt, &mut key)
        .map_err(|e| ClipboardError::encryption(format!("key derivation failed: {}", e)))?;

    Ok(key)
}

/// Test version uses a static salt for deterministic behavior.
#[cfg(test)]
fn derive_key(input: &[u8]) -> Result<[u8; 32]> {
    const TEST_SALT: &[u8] = b"photoncast-test-salt-deterministic";
    let mut key = [0u8; 32];

    let argon2 = Argon2::default();
    argon2
        .hash_password_into(input, TEST_SALT, &mut key)
        .map_err(|e| ClipboardError::encryption(format!("key derivation failed: {}", e)))?;

    Ok(key)
}

/// Gets the encryption salt from Keychain, or generates and stores a new one.
fn get_or_create_salt() -> Result<Vec<u8>> {
    // Try to retrieve existing salt from Keychain
    if let Some(salt) = get_salt_from_keychain()? {
        return Ok(salt);
    }
    
    // Generate new random salt
    let mut salt = vec![0u8; SALT_SIZE];
    rand::thread_rng().fill_bytes(&mut salt);
    
    // Store in Keychain for future use
    store_salt_in_keychain(&salt)?;
    
    tracing::info!("Generated and stored new encryption salt in Keychain");
    Ok(salt)
}

/// Retrieves the encryption salt from macOS Keychain.
#[cfg(target_os = "macos")]
fn get_salt_from_keychain() -> Result<Option<Vec<u8>>> {
    use std::process::Command;
    
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s", KEYCHAIN_SERVICE,
            "-a", KEYCHAIN_ACCOUNT,
            "-w", // Output password only
        ])
        .output()
        .map_err(|e| ClipboardError::encryption(format!("failed to access Keychain: {}", e)))?;
    
    if !output.status.success() {
        // Item not found is expected on first run
        return Ok(None);
    }
    
    let salt_hex = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if salt_hex.is_empty() {
        return Ok(None);
    }
    
    // Decode hex-encoded salt
    hex::decode(&salt_hex)
        .map(Some)
        .map_err(|e| ClipboardError::encryption(format!("invalid salt in Keychain: {}", e)))
}

/// Stores the encryption salt in macOS Keychain.
#[cfg(target_os = "macos")]
fn store_salt_in_keychain(salt: &[u8]) -> Result<()> {
    use std::process::Command;
    
    // Encode salt as hex for safe storage
    let salt_hex = hex::encode(salt);
    
    let output = Command::new("security")
        .args([
            "add-generic-password",
            "-s", KEYCHAIN_SERVICE,
            "-a", KEYCHAIN_ACCOUNT,
            "-w", &salt_hex,
            "-U", // Update if exists
        ])
        .output()
        .map_err(|e| ClipboardError::encryption(format!("failed to store in Keychain: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ClipboardError::encryption(format!("Keychain storage failed: {}", stderr)));
    }
    
    Ok(())
}

/// Non-macOS fallback: use a static salt (less secure but functional).
#[cfg(not(target_os = "macos"))]
fn get_salt_from_keychain() -> Result<Option<Vec<u8>>> {
    // Return static salt for non-macOS platforms
    Ok(Some(b"photoncast-clipboard-v1-fallback-salt".to_vec()))
}

#[cfg(not(target_os = "macos"))]
fn store_salt_in_keychain(_salt: &[u8]) -> Result<()> {
    // No-op for non-macOS platforms
    Ok(())
}

/// Generates a random nonce for AES-GCM.
fn generate_nonce() -> [u8; NONCE_SIZE] {
    let mut nonce = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce);
    nonce
}

/// Gets a machine-specific identifier.
///
/// This is used to derive the encryption key, ensuring encrypted data
/// can only be decrypted on the same machine.
fn get_machine_id() -> Result<String> {
    // On macOS, use the hardware UUID from IOKit
    #[cfg(target_os = "macos")]
    {
        get_macos_hardware_uuid()
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Fallback: use a combination of hostname and username
        let hostname = hostname::get().map_or_else(
            |_| "unknown-host".to_string(),
            |h| h.to_string_lossy().to_string(),
        );

        let username = std::env::var("USER").unwrap_or_else(|_| "unknown-user".to_string());

        Ok(format!("{}-{}", hostname, username))
    }
}

/// Gets the macOS hardware UUID.
#[cfg(target_os = "macos")]
fn get_macos_hardware_uuid() -> Result<String> {
    use std::process::Command;

    // Use system_profiler to get hardware UUID
    let output = Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .map_err(|e| ClipboardError::encryption(format!("failed to get hardware UUID: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the UUID from the output
    for line in stdout.lines() {
        if line.contains("IOPlatformUUID") {
            // Line format: "IOPlatformUUID" = "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"
            if let Some(uuid_start) = line
                .find('\"')
                .and_then(|i| line[i + 1..].find('\"').map(|j| i + j + 2))
            {
                if let Some(uuid_end) = line[uuid_start..].find('\"') {
                    return Ok(line[uuid_start..uuid_start + uuid_end].to_string());
                }
            }
        }
    }

    // Fallback: use a combination of hostname and user
    let hostname = hostname::get().map_or_else(
        |_| "unknown-host".to_string(),
        |h| h.to_string_lossy().to_string(),
    );

    let username = std::env::var("USER").unwrap_or_else(|_| "unknown-user".to_string());

    Ok(format!("fallback-{}-{}", hostname, username))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_roundtrip() {
        let manager = EncryptionManager::for_testing().expect("should create manager");

        let plaintext = b"Hello, World! This is a test message.";
        let encrypted = manager.encrypt(plaintext).expect("should encrypt");
        let decrypted = manager.decrypt(&encrypted).expect("should decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_string_encryption() {
        let manager = EncryptionManager::for_testing().expect("should create manager");

        let plaintext = "Hello, World! 🦀 Rust is awesome!";
        let encrypted = manager.encrypt_string(plaintext).expect("should encrypt");
        let decrypted = manager.decrypt_string(&encrypted).expect("should decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encryption_produces_different_output() {
        let manager = EncryptionManager::for_testing().expect("should create manager");

        let plaintext = b"Same message";
        let encrypted1 = manager.encrypt(plaintext).expect("should encrypt");
        let encrypted2 = manager.encrypt(plaintext).expect("should encrypt");

        // Different nonces should produce different ciphertext
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same plaintext
        let decrypted1 = manager.decrypt(&encrypted1).expect("should decrypt");
        let decrypted2 = manager.decrypt(&encrypted2).expect("should decrypt");
        assert_eq!(decrypted1, decrypted2);
    }

    #[test]
    fn test_empty_data() {
        let manager = EncryptionManager::for_testing().expect("should create manager");

        let plaintext = b"";
        let encrypted = manager.encrypt(plaintext).expect("should encrypt");
        let decrypted = manager.decrypt(&encrypted).expect("should decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_large_data() {
        let manager = EncryptionManager::for_testing().expect("should create manager");

        // 1MB of data
        let plaintext: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();
        let encrypted = manager.encrypt(&plaintext).expect("should encrypt");
        let decrypted = manager.decrypt(&encrypted).expect("should decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let manager = EncryptionManager::for_testing().expect("should create manager");

        // Too short
        let result = manager.decrypt(&[0u8; 5]);
        assert!(result.is_err());

        // Invalid ciphertext
        let mut invalid = vec![0u8; 20];
        rand::thread_rng().fill_bytes(&mut invalid);
        let result = manager.decrypt(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_different_keys_produce_different_output() {
        let manager1 = EncryptionManager::from_machine_id("machine-1").expect("should create");
        let manager2 = EncryptionManager::from_machine_id("machine-2").expect("should create");

        let plaintext = b"Test message";

        // Encrypt with manager1
        let encrypted = manager1.encrypt(plaintext).expect("should encrypt");

        // Trying to decrypt with manager2 should fail
        let result = manager2.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_same_machine_id_produces_same_key() {
        let manager1 = EncryptionManager::from_machine_id("same-machine").expect("should create");
        let manager2 = EncryptionManager::from_machine_id("same-machine").expect("should create");

        let plaintext = b"Test message";

        // Encrypt with manager1
        let encrypted = manager1.encrypt(plaintext).expect("should encrypt");

        // Decrypt with manager2 should succeed
        let decrypted = manager2.decrypt(&encrypted).expect("should decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_key_derivation_deterministic() {
        let key1 = derive_key(b"test-input").expect("should derive");
        let key2 = derive_key(b"test-input").expect("should derive");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_key_derivation_different_inputs() {
        let key1 = derive_key(b"input-1").expect("should derive");
        let key2 = derive_key(b"input-2").expect("should derive");
        assert_ne!(key1, key2);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_machine_id() {
        let id = get_machine_id().expect("should get machine ID");
        assert!(!id.is_empty());
        println!("Machine ID: {}", id);
    }
}
