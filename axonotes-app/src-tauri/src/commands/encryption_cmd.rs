use crate::crypto::bip39::{
    get_mnemonic, mnemonic_to_key, MNEMONIC_KEY_ENCRYPTION_CONTEXT, MNEMONIC_KEY_SIGNING_CONTEXT,
};
use crate::crypto::chacha::encrypt;
use crate::crypto::ed25519::generate_ed25519_keys;
use crate::crypto::hash::{
    derive_key, MASTER_PASSWORD_ENCRYPTION_CONTEXT, MASTER_PASSWORD_SIGNING_CONTEXT,
};
use crate::crypto::x25519::generate_x25519_keys;

#[tauri::command]
pub async fn set_master_password(password: String) -> Result<String, String> {
    let (private_encryption_key, public_encryption_key) = generate_x25519_keys();
    let (private_signing_key, public_signing_key) = generate_ed25519_keys();

    let pwd_encryption_key = derive_key(password.as_str(), MASTER_PASSWORD_ENCRYPTION_CONTEXT);
    let pwd_signing_key = derive_key(password.as_str(), MASTER_PASSWORD_SIGNING_CONTEXT);

    let mnemonic = get_mnemonic().unwrap();
    let mnemonic_encryption_key =
        mnemonic_to_key(mnemonic.as_str(), MNEMONIC_KEY_ENCRYPTION_CONTEXT);
    let mnemonic_signing_key = mnemonic_to_key(mnemonic.as_str(), MNEMONIC_KEY_SIGNING_CONTEXT);

    let pwd_encrypted_private_encryption_key = encrypt(
        pwd_encryption_key.as_slice(),
        private_encryption_key.as_slice(),
    )
    .unwrap();
    let pwd_encrypted_private_signing_key =
        encrypt(pwd_signing_key.as_slice(), private_signing_key.as_slice()).unwrap();

    let mnemonic_encrypted_private_encryption_key = encrypt(
        mnemonic_encryption_key.as_slice(),
        private_encryption_key.as_slice(),
    )
    .unwrap();
    let mnemonic_encrypted_private_signing_key = encrypt(
        mnemonic_signing_key.as_slice(),
        private_signing_key.as_slice(),
    )
    .unwrap();

    // TODO: store keys

    Ok(mnemonic)
}
