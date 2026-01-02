use spacetimedb::Identity;

#[spacetimedb::table(name = private_user)]
pub struct User {
    #[primary_key]
    pub identity: Identity,

    // Asymmetric Encryption
    pub public_encryption_key: Vec<u8>,
    pub pwd_encrypted_private_encryption_key: Vec<u8>,
    pub mnemonic_encrypted_private_encryption_key: Vec<u8>,

    // Signing
    pub public_signing_key: Vec<u8>,
    pub pwd_encrypted_private_signing_key: Vec<u8>,
    pub mnemonic_encrypted_private_signing_key: Vec<u8>,
}
