use crate::core::error::Result;
use std::collections::HashMap;
use std::sync::Mutex;

pub trait KeyStorage: Send + Sync {
    fn store_identity(&self, key: &[u8], id: &str) -> Result<()>;
    fn load_identity(&self, id: &str) -> Result<Vec<u8>>;
    fn delete_identity(&self, id: &str) -> Result<()>;
}

pub struct MemoryKeyStorage {
    keys: Mutex<HashMap<String, Vec<u8>>>,
}

impl MemoryKeyStorage {
    pub fn new() -> Self {
        Self {
            keys: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryKeyStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyStorage for MemoryKeyStorage {
    fn store_identity(&self, key: &[u8], id: &str) -> Result<()> {
        let mut keys = self.keys.lock().unwrap();
        keys.insert(id.to_string(), key.to_vec());
        Ok(())
    }
    
    fn load_identity(&self, id: &str) -> Result<Vec<u8>> {
        let keys = self.keys.lock().unwrap();
        keys.get(id)
            .cloned()
            .ok_or(crate::core::error::NoiseError::InvalidParameter)
    }
    
    fn delete_identity(&self, id: &str) -> Result<()> {
        let mut keys = self.keys.lock().unwrap();
        keys.remove(id);
        Ok(())
    }
}