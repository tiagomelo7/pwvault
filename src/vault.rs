use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroize;

use crate::crypto;

#[derive(Serialize, Deserialize, Clone)]
pub struct Entry {
    pub id: Uuid,
    pub site: String,
    pub username: String,
    pub password: String,
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_round_trips_through_json() {
        let entry = Entry {
            id: Uuid::new_v4(),
            site: "example.com".to_string(),
            username: "alice".to_string(),
            password: "secret".to_string(),
            notes: Some("hello".to_string()),
        };

        let bytes = serde_json::to_vec(&entry).expect("serialização da entrada");
        let decoded: Entry = serde_json::from_slice(&bytes).expect("deserialização da entrada");

        assert_eq!(decoded.id, entry.id);
        assert_eq!(decoded.site, entry.site);
        assert_eq!(decoded.username, entry.username);
        assert_eq!(decoded.password, entry.password);
        assert_eq!(decoded.notes, entry.notes);
    }

    #[test]
    fn strong_passwords_are_accepted() {
        assert!(validate_master_password("Str0ng!PassWord").is_ok());
    }

    #[test]
    fn weak_passwords_are_rejected() {
        assert!(validate_master_password("teste").is_err());
        assert!(validate_master_password("12345678").is_err());
        assert!(validate_master_password("password").is_err());
        assert!(validate_master_password("Abcdef1!").is_err());
    }

    #[test]
    fn removing_entry_from_session_works() {
        let mut session = VaultSession {
            key: [0u8; 32],
            salt: vec![],
            entries: vec![
                Entry {
                    id: Uuid::new_v4(),
                    site: "example.com".to_string(),
                    username: "alice".to_string(),
                    password: "secret".to_string(),
                    notes: None,
                }
            ],
        };

        let id = session.entries[0].id;
        remove_entry(&mut session, id).expect("deveria remover a entrada");

        assert!(session.entries.is_empty());
    }

    #[test]
    fn changing_master_password_reencrypts_vault() {
        let dir = std::env::temp_dir().join(format!("pwvault-rotate-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vault.dat");

        crate::storage::write_vault_file(
            &path,
            &serde_json::to_vec(&create_new_vault(b"OldPass!123").1).unwrap(),
        )
        .unwrap();

        change_master_password(&path, "OldPass!123", "NewPass!456").unwrap();

        assert!(open_vault(b"NewPass!456", &std::fs::read(&path).unwrap()).is_ok());
        assert!(open_vault(b"OldPass!123", &std::fs::read(&path).unwrap()).is_err());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[derive(Serialize, Deserialize)]
pub struct VaultFile {
    pub salt: Vec<u8>,
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

pub struct VaultSession {
    pub key: [u8; 32],
    pub salt: Vec<u8>,
    pub entries: Vec<Entry>,
}

impl VaultSession {
    pub fn clear_sensitive_data(&mut self) {
        self.key.zeroize();

        for entry in &mut self.entries {
            entry.password.zeroize();
            if let Some(notes) = &mut entry.notes {
                notes.zeroize();
            }
        }
    }
}

impl Drop for VaultSession {
    fn drop(&mut self) {
        self.clear_sensitive_data();
    }
}

#[derive(Debug)]
pub enum VaultError {
    WrongPassword,
    WeakPassword(String),
    EntryNotFound,
    Io(std::io::Error),
    Serialization(serde_json::Error),
}

pub fn validate_master_password(password: &str) -> Result<(), VaultError> {
    if password.len() < 12 {
        return Err(VaultError::WeakPassword(
            "A senha deve ter pelo menos 12 caracteres.".to_string(),
        ));
    }

    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password.chars().any(|c| !c.is_alphanumeric());

    if !(has_upper && has_lower && has_digit && has_symbol) {
        return Err(VaultError::WeakPassword(
            "A senha deve conter maiúscula, minúscula, número e símbolo.".to_string(),
        ));
    }

    let normalized = password.to_ascii_lowercase();
    let common = [
        "123456", "12345678", "password", "senha", "qwerty", "admin", "teste", "welcome",
    ];
    if common.iter().any(|candidate| normalized.contains(candidate)) {
        return Err(VaultError::WeakPassword(
            "A senha não pode conter palavras ou sequências comuns.".to_string(),
        ));
    }

    Ok(())
}

pub fn create_new_vault(password: &[u8]) -> (VaultSession, VaultFile) {
    let salt_array = crypto::generate_salt();
    let key = crypto::derive_key(password, &salt_array);
    let entries: Vec<Entry> = Vec::new();

    let dados_puros = serde_json::to_vec(&entries).expect("falha ao criar formato inicial do vault");
    let (nonce, ciphertext) = crypto::encrypt(&key, &dados_puros);

    let vault_file = VaultFile {
        salt: salt_array.to_vec(),
        nonce,
        ciphertext,
    };

    let vault_session = VaultSession {
        key,
        salt: salt_array.to_vec(),
        entries,
    };

    (vault_session, vault_file)
}

pub fn open_vault(password: &[u8], file_bytes: &[u8]) -> Result<VaultSession, VaultError> {
    let vault_file: VaultFile =
        serde_json::from_slice(file_bytes).map_err(VaultError::Serialization)?;

    let key = crypto::derive_key(password, &vault_file.salt);

    let plaintext = crypto::decrypt(&key, &vault_file.nonce, &vault_file.ciphertext)
        .map_err(|_| VaultError::WrongPassword)?;

    let entries: Vec<Entry> =
        serde_json::from_slice(&plaintext).map_err(VaultError::Serialization)?;

    Ok(VaultSession {
        key,
        salt: vault_file.salt,
        entries,
    })
}

pub fn open_or_create(path: &std::path::Path, password: &str) -> Result<VaultSession, VaultError> {
    match std::fs::read(path) {
        Ok(file_bytes) if !file_bytes.is_empty() => {
            open_vault(password.as_bytes(), &file_bytes)
        }
        Ok(_) | Err(_) => {
            validate_master_password(password)?;

            let (session, vault_file) = create_new_vault(password.as_bytes());
            let bytes = serde_json::to_vec(&vault_file).map_err(VaultError::Serialization)?;
            if let Err(error) = std::fs::create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new("."))) {
                return Err(VaultError::Io(error));
            }
            if let Err(error) = std::fs::write(path, &bytes) {
                return Err(VaultError::Io(error));
            }
            Ok(session)
        }
    }
}

pub fn change_master_password(path: &std::path::Path, current_password: &str, new_password: &str) -> Result<(), VaultError> {
    validate_master_password(new_password)?;

    let file_bytes = std::fs::read(path).map_err(VaultError::Io)?;
    let mut session = open_vault(current_password.as_bytes(), &file_bytes)?;

    let new_salt = crypto::generate_salt();
    let new_key = crypto::derive_key(new_password.as_bytes(), &new_salt);
    let plaintext = serde_json::to_vec(&session.entries).map_err(VaultError::Serialization)?;
    let (nonce, ciphertext) = crypto::encrypt(&new_key, &plaintext);

    let updated = VaultFile {
        salt: new_salt.to_vec(),
        nonce,
        ciphertext,
    };

    let payload = serde_json::to_vec(&updated).map_err(VaultError::Serialization)?;
    crate::storage::write_vault_file(path, &payload).map_err(VaultError::Io)?;

    session.clear_sensitive_data();
    Ok(())
}

pub fn save_vault(session: &VaultSession) -> Result<Vec<u8>, VaultError> {
    let plaintext = serde_json::to_vec(&session.entries).map_err(VaultError::Serialization)?;
    let (nonce, ciphertext) = crypto::encrypt(&session.key, &plaintext);

    let vault_file = VaultFile {
        salt: session.salt.clone(),
        nonce,
        ciphertext,
    };

    serde_json::to_vec(&vault_file).map_err(VaultError::Serialization)
}

pub fn add_entry(session: &mut VaultSession, entry: Entry) {
    session.entries.push(entry);
}

pub fn update_entry(session: &mut VaultSession, updated: Entry) -> Result<(), VaultError> {
    let entry = session
        .entries
        .iter_mut()
        .find(|entry| entry.id == updated.id)
        .ok_or(VaultError::EntryNotFound)?;

    *entry = updated;

    Ok(())
}

pub fn remove_entry(session: &mut VaultSession, id: Uuid) -> Result<(), VaultError> {
    if let Some(index) = session.entries.iter().position(|entry| entry.id == id) {
        session.entries.remove(index);
        Ok(())
    } else {
        Err(VaultError::EntryNotFound)
    }
}

pub fn find_entry(session: &VaultSession, id: Uuid) -> Option<&Entry> {
    for entry in &session.entries {
        if entry.id == id {
            return Some(entry);
        }
    }
    None
}