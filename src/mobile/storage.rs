//! Key storage abstraction for mobile platforms

use crate::core::error::{NoiseError, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zeroize::Zeroize;

/// Trait for secure key storage on mobile platforms
pub trait KeyStorage: Send + Sync {
    /// Store an identity key with a given identifier
    fn store_identity(&self, key: &[u8], id: &str) -> Result<()>;
    
    /// Load an identity key by identifier
    fn load_identity(&self, id: &str) -> Result<Vec<u8>>;
    
    /// Delete an identity key by identifier
    fn delete_identity(&self, id: &str) -> Result<()>;
    
    /// List all stored identity identifiers
    fn list_identities(&self) -> Result<Vec<String>>;
    
    /// Check if an identity exists
    fn has_identity(&self, id: &str) -> Result<bool>;
    
    /// Store session keys for later resumption
    fn store_session(&self, session_id: &str, session_data: &[u8]) -> Result<()>;
    
    /// Load session data
    fn load_session(&self, session_id: &str) -> Result<Vec<u8>>;
    
    /// Delete session data
    fn delete_session(&self, session_id: &str) -> Result<()>;
}

/// Secure memory storage for keys (for testing and development)
#[derive(Clone)]
pub struct MemoryKeyStorage {
    keys: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    sessions: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MemoryKeyStorage {
    /// Create a new memory key storage
    pub fn new() -> Self {
        Self {
            keys: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Clear all stored keys and sessions
    pub fn clear(&self) -> Result<()> {
        let mut keys = self.keys.lock().map_err(|_| NoiseError::InvalidState("Lock poisoned".to_string()))?;
        let mut sessions = self.sessions.lock().map_err(|_| NoiseError::InvalidState("Lock poisoned".to_string()))?;
        
        // Zeroize all keys before clearing
        for (_, mut key) in keys.drain() {
            key.zeroize();
        }
        
        for (_, mut session) in sessions.drain() {
            session.zeroize();
        }
        
        Ok(())
    }
}

impl Default for MemoryKeyStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MemoryKeyStorage {
    fn drop(&mut self) {
        // Try to clear on drop, ignore errors
        let _ = self.clear();
    }
}

impl KeyStorage for MemoryKeyStorage {
    fn store_identity(&self, key: &[u8], id: &str) -> Result<()> {
        if key.len() != 32 {
            return Err(NoiseError::InvalidParameter);
        }
        
        let mut keys = self.keys.lock().map_err(|_| NoiseError::InvalidState("Lock poisoned".to_string()))?;
        
        // Zeroize old key if it exists
        if let Some(mut old_key) = keys.remove(id) {
            old_key.zeroize();
        }
        
        keys.insert(id.to_string(), key.to_vec());
        Ok(())
    }
    
    fn load_identity(&self, id: &str) -> Result<Vec<u8>> {
        let keys = self.keys.lock().map_err(|_| NoiseError::InvalidState("Lock poisoned".to_string()))?;
        keys.get(id)
            .cloned()
            .ok_or(NoiseError::InvalidParameter)
    }
    
    fn delete_identity(&self, id: &str) -> Result<()> {
        let mut keys = self.keys.lock().map_err(|_| NoiseError::InvalidState("Lock poisoned".to_string()))?;
        if let Some(mut key) = keys.remove(id) {
            key.zeroize();
        }
        Ok(())
    }
    
    fn list_identities(&self) -> Result<Vec<String>> {
        let keys = self.keys.lock().map_err(|_| NoiseError::InvalidState("Lock poisoned".to_string()))?;
        Ok(keys.keys().cloned().collect())
    }
    
    fn has_identity(&self, id: &str) -> Result<bool> {
        let keys = self.keys.lock().map_err(|_| NoiseError::InvalidState("Lock poisoned".to_string()))?;
        Ok(keys.contains_key(id))
    }
    
    fn store_session(&self, session_id: &str, session_data: &[u8]) -> Result<()> {
        let mut sessions = self.sessions.lock().map_err(|_| NoiseError::InvalidState("Lock poisoned".to_string()))?;
        
        // Zeroize old session if it exists
        if let Some(mut old_session) = sessions.remove(session_id) {
            old_session.zeroize();
        }
        
        sessions.insert(session_id.to_string(), session_data.to_vec());
        Ok(())
    }
    
    fn load_session(&self, session_id: &str) -> Result<Vec<u8>> {
        let sessions = self.sessions.lock().map_err(|_| NoiseError::InvalidState("Lock poisoned".to_string()))?;
        sessions.get(session_id)
            .cloned()
            .ok_or(NoiseError::InvalidParameter)
    }
    
    fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().map_err(|_| NoiseError::InvalidState("Lock poisoned".to_string()))?;
        if let Some(mut session) = sessions.remove(session_id) {
            session.zeroize();
        }
        Ok(())
    }
}

/// iOS Keychain storage (placeholder for actual implementation)
#[cfg(target_os = "ios")]
pub struct KeychainStorage;

#[cfg(target_os = "ios")]
impl KeychainStorage {
    /// Create a new Keychain storage instance
    pub fn new() -> Self {
        KeychainStorage
    }
}

#[cfg(target_os = "ios")]
impl KeyStorage for KeychainStorage {
    fn store_identity(&self, _key: &[u8], _id: &str) -> Result<()> {
        // TODO: Implement using Security framework
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn load_identity(&self, _id: &str) -> Result<Vec<u8>> {
        // TODO: Implement using Security framework
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn delete_identity(&self, _id: &str) -> Result<()> {
        // TODO: Implement using Security framework
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn list_identities(&self) -> Result<Vec<String>> {
        // TODO: Implement using Security framework
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn has_identity(&self, _id: &str) -> Result<bool> {
        // TODO: Implement using Security framework
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn store_session(&self, _session_id: &str, _session_data: &[u8]) -> Result<()> {
        // TODO: Implement using Security framework
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn load_session(&self, _session_id: &str) -> Result<Vec<u8>> {
        // TODO: Implement using Security framework
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn delete_session(&self, _session_id: &str) -> Result<()> {
        // TODO: Implement using Security framework
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
}

/// Android Keystore storage (placeholder for actual implementation)
#[cfg(target_os = "android")]
pub struct KeystoreStorage;

#[cfg(target_os = "android")]
impl KeystoreStorage {
    /// Create a new Keystore storage instance
    pub fn new() -> Self {
        KeystoreStorage
    }
}

#[cfg(target_os = "android")]
impl KeyStorage for KeystoreStorage {
    fn store_identity(&self, _key: &[u8], _id: &str) -> Result<()> {
        // TODO: Implement using Android Keystore
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn load_identity(&self, _id: &str) -> Result<Vec<u8>> {
        // TODO: Implement using Android Keystore
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn delete_identity(&self, _id: &str) -> Result<()> {
        // TODO: Implement using Android Keystore
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn list_identities(&self) -> Result<Vec<String>> {
        // TODO: Implement using Android Keystore
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn has_identity(&self, _id: &str) -> Result<bool> {
        // TODO: Implement using Android Keystore
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn store_session(&self, _session_id: &str, _session_data: &[u8]) -> Result<()> {
        // TODO: Implement using Android Keystore
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn load_session(&self, _session_id: &str) -> Result<Vec<u8>> {
        // TODO: Implement using Android Keystore
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
    
    fn delete_session(&self, _session_id: &str) -> Result<()> {
        // TODO: Implement using Android Keystore
        Err(NoiseError::InvalidState("Not implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_storage_basic() {
        let storage = MemoryKeyStorage::new();
        let key = vec![0u8; 32];
        let id = "test_key";
        
        // Store key
        storage.store_identity(&key, id).unwrap();
        
        // Check it exists
        assert!(storage.has_identity(id).unwrap());
        
        // Load key
        let loaded = storage.load_identity(id).unwrap();
        assert_eq!(key, loaded);
        
        // List identities
        let identities = storage.list_identities().unwrap();
        assert_eq!(identities.len(), 1);
        assert!(identities.contains(&id.to_string()));
        
        // Delete key
        storage.delete_identity(id).unwrap();
        assert!(!storage.has_identity(id).unwrap());
    }
    
    #[test]
    fn test_memory_storage_sessions() {
        let storage = MemoryKeyStorage::new();
        let session_data = vec![1u8, 2, 3, 4, 5];
        let session_id = "test_session";
        
        // Store session
        storage.store_session(session_id, &session_data).unwrap();
        
        // Load session
        let loaded = storage.load_session(session_id).unwrap();
        assert_eq!(session_data, loaded);
        
        // Delete session
        storage.delete_session(session_id).unwrap();
        assert!(storage.load_session(session_id).is_err());
    }
    
    #[test]
    fn test_invalid_key_size() {
        let storage = MemoryKeyStorage::new();
        let invalid_key = vec![0u8; 16]; // Wrong size
        
        assert!(storage.store_identity(&invalid_key, "test").is_err());
    }
    
    #[test]
    fn test_zeroize_on_delete() {
        let storage = MemoryKeyStorage::new();
        let key = vec![42u8; 32];
        let id = "zeroize_test";
        
        storage.store_identity(&key, id).unwrap();
        storage.delete_identity(id).unwrap();
        
        // Key should be gone
        assert!(storage.load_identity(id).is_err());
    }
}