use crate::core::error::{NoiseError, Result};
use crate::core::session::NoiseSession;

pub struct ResilientSession {
    inner: NoiseSession,
    last_sent: u64,
    last_received: u64,
}

impl ResilientSession {
    pub fn new(session: NoiseSession) -> Self {
        Self {
            inner: session,
            last_sent: 0,
            last_received: 0,
        }
    }
}