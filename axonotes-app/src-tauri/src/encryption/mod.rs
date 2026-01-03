use crate::crypto::bip39::{
    mnemonic_to_key, MNEMONIC_KEY_ENCRYPTION_CONTEXT, MNEMONIC_KEY_SIGNING_CONTEXT,
};
use crate::crypto::chacha::{decrypt, encrypt};
use crate::crypto::ed25519::{generate_ed25519_keys, sign_message};
use crate::crypto::hash::{
    derive_key, MASTER_PASSWORD_ENCRYPTION_CONTEXT, MASTER_PASSWORD_SIGNING_CONTEXT,
};
use crate::crypto::x25519::generate_x25519_keys;
use crate::database::keys::Keys;
use crate::stdb::context::get_message_to_sign;
use crate::stdb_bindings::User;

#[derive(Clone)]
pub struct UserKeysDecrypted {
    pub public_encryption_key: Vec<u8>,
    pub private_encryption_key: Vec<u8>,
    pub public_signing_key: Vec<u8>,
    pub private_signing_key: Vec<u8>,
}

impl From<Keys> for UserKeysDecrypted {
    fn from(keys: Keys) -> Self {
        Self {
            public_encryption_key: keys.public_encryption_key,
            private_encryption_key: keys.private_encryption_key,
            public_signing_key: keys.public_signing_key,
            private_signing_key: keys.private_signing_key,
        }
    }
}

#[derive(Clone)]
pub struct UserKeysEncrypted {
    pub public_encryption_key: Vec<u8>,
    pub pwd_encrypted_private_encryption_key: Vec<u8>,
    pub mnemonic_encrypted_private_encryption_key: Vec<u8>,
    pub public_signing_key: Vec<u8>,
    pub pwd_encrypted_private_signing_key: Vec<u8>,
    pub mnemonic_encrypted_private_signing_key: Vec<u8>,
}

impl From<User> for UserKeysEncrypted {
    fn from(user: User) -> Self {
        Self {
            public_encryption_key: user.public_encryption_key,
            pwd_encrypted_private_encryption_key: user.pwd_encrypted_private_encryption_key,
            mnemonic_encrypted_private_encryption_key: user
                .mnemonic_encrypted_private_encryption_key,
            public_signing_key: user.public_signing_key,
            pwd_encrypted_private_signing_key: user.pwd_encrypted_private_signing_key,
            mnemonic_encrypted_private_signing_key: user.mnemonic_encrypted_private_signing_key,
        }
    }
}

pub struct UserKeysBoth {
    pub public_encryption_key: Vec<u8>,
    pub private_encryption_key: Vec<u8>,
    pub pwd_encrypted_private_encryption_key: Vec<u8>,
    pub mnemonic_encrypted_private_encryption_key: Vec<u8>,
    pub public_signing_key: Vec<u8>,
    pub private_signing_key: Vec<u8>,
    pub pwd_encrypted_private_signing_key: Vec<u8>,
    pub mnemonic_encrypted_private_signing_key: Vec<u8>,
}

pub fn generate_decrypted_keys() -> UserKeysDecrypted {
    let (private_encryption_key, public_encryption_key) = generate_x25519_keys();
    let (private_signing_key, public_signing_key) = generate_ed25519_keys();

    UserKeysDecrypted {
        public_encryption_key: public_encryption_key.to_vec(),
        private_encryption_key: private_encryption_key.to_vec(),
        public_signing_key: public_signing_key.to_vec(),
        private_signing_key: private_signing_key.to_vec(),
    }
}

pub fn decrypted_to_encrypted(
    user_keys_decrypted: UserKeysDecrypted,
    password: String,
    mnemonic: String,
) -> Result<UserKeysEncrypted, String> {
    let pwd_encryption_key = derive_key(password.as_str(), MASTER_PASSWORD_ENCRYPTION_CONTEXT);
    let pwd_signing_key = derive_key(password.as_str(), MASTER_PASSWORD_SIGNING_CONTEXT);
    let mnemonic_encryption_key =
        mnemonic_to_key(mnemonic.as_str(), MNEMONIC_KEY_ENCRYPTION_CONTEXT);
    let mnemonic_signing_key = mnemonic_to_key(mnemonic.as_str(), MNEMONIC_KEY_SIGNING_CONTEXT);

    // Encrypt Keys
    let pwd_encrypted_private_encryption_key = encrypt(
        pwd_encryption_key.as_slice(),
        user_keys_decrypted.private_encryption_key.as_slice(),
    )
    .map_err(|e| format!("Failed to encrypt encryption key with pwd: {}", e))?;
    let pwd_encrypted_private_signing_key = encrypt(
        pwd_signing_key.as_slice(),
        user_keys_decrypted.private_signing_key.as_slice(),
    )
    .map_err(|e| format!("Failed to encrypt signing key with pwd: {}", e))?;

    let mnemonic_encrypted_private_encryption_key = encrypt(
        mnemonic_encryption_key.as_slice(),
        user_keys_decrypted.private_encryption_key.as_slice(),
    )
    .map_err(|e| format!("Failed to encrypt encryption key with mnemonic: {}", e))?;
    let mnemonic_encrypted_private_signing_key = encrypt(
        mnemonic_signing_key.as_slice(),
        user_keys_decrypted.private_signing_key.as_slice(),
    )
    .map_err(|e| format!("Failed to encrypt signing key with mnemonic: {}", e))?;

    Ok(UserKeysEncrypted {
        public_encryption_key: user_keys_decrypted.public_encryption_key,
        pwd_encrypted_private_encryption_key,
        mnemonic_encrypted_private_encryption_key,
        public_signing_key: user_keys_decrypted.public_signing_key,
        pwd_encrypted_private_signing_key,
        mnemonic_encrypted_private_signing_key,
    })
}

pub fn pwd_encrypted_to_decrypted(
    user_keys_encrypted: UserKeysEncrypted,
    password: String,
) -> Result<UserKeysDecrypted, String> {
    let pwd_encryption_key = derive_key(password.as_str(), MASTER_PASSWORD_ENCRYPTION_CONTEXT);
    let pwd_signing_key = derive_key(password.as_str(), MASTER_PASSWORD_SIGNING_CONTEXT);

    // Get current user keys
    let public_encryption_key = user_keys_encrypted.public_encryption_key;
    let pwd_encrypted_private_encryption_key =
        user_keys_encrypted.pwd_encrypted_private_encryption_key;
    let public_signing_key = user_keys_encrypted.public_signing_key;
    let pwd_encrypted_private_signing_key = user_keys_encrypted.pwd_encrypted_private_signing_key;

    // Decrypt the private keys
    let private_encryption_key = decrypt(
        pwd_encryption_key.as_slice(),
        pwd_encrypted_private_encryption_key.as_slice(),
    )
    .map_err(|e| format!("Failed to decrypt encryption key: {}", e))?;

    let private_signing_key = decrypt(
        pwd_signing_key.as_slice(),
        pwd_encrypted_private_signing_key.as_slice(),
    )
    .map_err(|e| format!("Failed to decrypt signing key: {}", e))?;

    Ok(UserKeysDecrypted {
        public_encryption_key,
        private_encryption_key,
        public_signing_key,
        private_signing_key,
    })
}

pub fn mnemonic_encrypted_to_decrypted(
    user_keys_encrypted: UserKeysEncrypted,
    mnemonic: String,
) -> Result<UserKeysDecrypted, String> {
    let mnemonic_encryption_key =
        mnemonic_to_key(mnemonic.as_str(), MNEMONIC_KEY_ENCRYPTION_CONTEXT);
    let mnemonic_signing_key = mnemonic_to_key(mnemonic.as_str(), MNEMONIC_KEY_SIGNING_CONTEXT);

    // Get current user keys
    let public_encryption_key = user_keys_encrypted.public_encryption_key;
    let mnemonic_encrypted_private_encryption_key =
        user_keys_encrypted.mnemonic_encrypted_private_encryption_key;
    let public_signing_key = user_keys_encrypted.public_signing_key;
    let mnemonic_encrypted_private_signing_key =
        user_keys_encrypted.mnemonic_encrypted_private_signing_key;

    // Decrypt the private keys
    let private_encryption_key = decrypt(
        mnemonic_encryption_key.as_slice(),
        mnemonic_encrypted_private_encryption_key.as_slice(),
    )
    .map_err(|e| format!("Failed to decrypt encryption key: {}", e))?;

    let private_signing_key = decrypt(
        mnemonic_signing_key.as_slice(),
        mnemonic_encrypted_private_signing_key.as_slice(),
    )
    .map_err(|e| format!("Failed to decrypt signing key: {}", e))?;

    Ok(UserKeysDecrypted {
        public_encryption_key,
        private_encryption_key,
        public_signing_key,
        private_signing_key,
    })
}

pub fn sign_encryption_update(
    decrypted_keys: &UserKeysDecrypted,
    encrypted_keys: &UserKeysEncrypted,
) -> Result<Vec<u8>, String> {
    let message = get_message_to_sign(encrypted_keys.clone());
    let private_signing_key_array: &[u8; 32] = decrypted_keys
        .private_signing_key
        .as_slice()
        .try_into()
        .map_err(|_| "Private signing key must be exactly 32 bytes")?;

    let signature: [u8; 64] = sign_message(private_signing_key_array, message.as_slice())
        .map_err(|e| format!("Failed to sign encryption update: {}", e))?;

    Ok(signature.to_vec())
}

pub fn reencrypt_with_new_password(
    user_keys_decrypted: UserKeysDecrypted,
    old_encrypted: UserKeysEncrypted,
    new_password: String,
) -> Result<UserKeysEncrypted, String> {
    let pwd_encryption_key = derive_key(new_password.as_str(), MASTER_PASSWORD_ENCRYPTION_CONTEXT);
    let pwd_signing_key = derive_key(new_password.as_str(), MASTER_PASSWORD_SIGNING_CONTEXT);

    // Encrypt only with new password
    let pwd_encrypted_private_encryption_key = encrypt(
        pwd_encryption_key.as_slice(),
        user_keys_decrypted.private_encryption_key.as_slice(),
    )
    .map_err(|e| format!("Failed to encrypt encryption key with pwd: {}", e))?;

    let pwd_encrypted_private_signing_key = encrypt(
        pwd_signing_key.as_slice(),
        user_keys_decrypted.private_signing_key.as_slice(),
    )
    .map_err(|e| format!("Failed to encrypt signing key with pwd: {}", e))?;

    // Return with new password encryption but keep old mnemonic encryption
    Ok(UserKeysEncrypted {
        public_encryption_key: old_encrypted.public_encryption_key,
        pwd_encrypted_private_encryption_key,
        mnemonic_encrypted_private_encryption_key: old_encrypted
            .mnemonic_encrypted_private_encryption_key,
        public_signing_key: old_encrypted.public_signing_key,
        pwd_encrypted_private_signing_key,
        mnemonic_encrypted_private_signing_key: old_encrypted
            .mnemonic_encrypted_private_signing_key,
    })
}

pub fn reencrypt_with_new_mnemonic(
    user_keys_decrypted: UserKeysDecrypted,
    old_encrypted: UserKeysEncrypted,
    new_mnemonic: String,
) -> Result<UserKeysEncrypted, String> {
    let mnemonic_encryption_key =
        mnemonic_to_key(new_mnemonic.as_str(), MNEMONIC_KEY_ENCRYPTION_CONTEXT);
    let mnemonic_signing_key = mnemonic_to_key(new_mnemonic.as_str(), MNEMONIC_KEY_SIGNING_CONTEXT);

    // Encrypt only with new mnemonic
    let mnemonic_encrypted_private_encryption_key = encrypt(
        mnemonic_encryption_key.as_slice(),
        user_keys_decrypted.private_encryption_key.as_slice(),
    )
    .map_err(|e| format!("Failed to encrypt encryption key with mnemonic: {}", e))?;

    let mnemonic_encrypted_private_signing_key = encrypt(
        mnemonic_signing_key.as_slice(),
        user_keys_decrypted.private_signing_key.as_slice(),
    )
    .map_err(|e| format!("Failed to encrypt signing key with mnemonic: {}", e))?;

    // Return with new mnemonic encryption but keep old password encryption
    Ok(UserKeysEncrypted {
        public_encryption_key: old_encrypted.public_encryption_key,
        pwd_encrypted_private_encryption_key: old_encrypted.pwd_encrypted_private_encryption_key,
        mnemonic_encrypted_private_encryption_key,
        public_signing_key: old_encrypted.public_signing_key,
        pwd_encrypted_private_signing_key: old_encrypted.pwd_encrypted_private_signing_key,
        mnemonic_encrypted_private_signing_key,
    })
}
