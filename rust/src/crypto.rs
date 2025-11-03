use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};

/// Encrypt data using XChaCha20-Poly1305
pub fn seal_xchacha(key: &[u8; 32], nonce: &[u8; 24], plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    #[allow(deprecated)]
    cipher
        .encrypt(XNonce::from_slice(nonce), plaintext)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))
}

/// Decrypt data using XChaCha20-Poly1305
pub fn open_xchacha(
    key: &[u8; 32],
    nonce: &[u8; 24],
    ciphertext: &[u8],
) -> anyhow::Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    #[allow(deprecated)]
    cipher
        .decrypt(XNonce::from_slice(nonce), ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [42u8; 32];
        let nonce = [13u8; 24];
        let plaintext = b"Secret message";

        let ciphertext = seal_xchacha(&key, &nonce, plaintext).unwrap();
        let decrypted = open_xchacha(&key, &nonce, &ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }
}
