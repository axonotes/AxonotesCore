use crate::crypto::bip39::{
    get_mnemonic, mnemonic_to_key, MNEMONIC_KEY_ENCRYPTION_CONTEXT, MNEMONIC_KEY_SIGNING_CONTEXT,
};
use crate::crypto::chacha::encrypt;
use crate::crypto::ed25519::generate_ed25519_keys;
use crate::crypto::hash::{
    derive_key, MASTER_PASSWORD_ENCRYPTION_CONTEXT, MASTER_PASSWORD_SIGNING_CONTEXT,
};
use crate::crypto::x25519::generate_x25519_keys;
use crate::database::keys::Keys;
use crate::{database, stdb};

/// This function can only be called once per active user since it creates a new profile on the stdb instance
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

    // Store in local db
    database::save_active_user_keys(Keys {
        user_id: "".to_string(), // This is allowed to be empty since 'save_active_user_keys' overwrites with active user id
        public_encryption_key: public_encryption_key.to_vec(),
        private_encryption_key: private_encryption_key.to_vec(),
        public_signing_key: public_signing_key.to_vec(),
        private_signing_key: private_signing_key.to_vec(),
    })
    .await?;

    // Store on stdb
    stdb::active_profile()
        .create_user(
            public_encryption_key.to_vec(),
            pwd_encrypted_private_encryption_key,
            mnemonic_encrypted_private_encryption_key,
            public_signing_key.to_vec(),
            pwd_encrypted_private_signing_key,
            mnemonic_encrypted_private_signing_key,
        )
        .await?;

    Ok(mnemonic)
}
