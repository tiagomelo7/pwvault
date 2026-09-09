use rand::{rngs::OsRng, RngCore};
use argon2::Argon2;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};

const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;

pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

pub fn derive_key(password: &[u8], salt: &[u8]) -> [u8; KEY_LEN] {
    let argon2 = Argon2::default();

    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password, salt, &mut key)
        .expect("erro ao derivar a chave Argon2");

    key
}

pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> ([u8; 12], Vec<u8>) {
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);

    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .expect("falha ao criptografar dados do cofre");

    (nonce_bytes, ciphertext)
}

pub fn decrypt(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>, aes_gcm::Error> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce_formatado = Nonce::from_slice(nonce);
    cipher.decrypt(nonce_formatado, ciphertext)
}