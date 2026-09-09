use std::path::PathBuf;
use crate::vault::VaultSession;

pub struct AppState {
    pub session: Option<VaultSession>,
    pub vault_path: PathBuf,
}

impl AppState {
    pub fn new(vault_path: PathBuf) -> Self {
        Self {
            session: None,
            vault_path,
        }
    }
}
pub fn is_unlocked(state: &AppState) -> bool {
    state.session.is_some()
}