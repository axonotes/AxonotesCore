use spacetimedb::{table, Identity};

#[table(name = user, public)]
pub struct User {
    #[primary_key]
    pub identity: Identity,

    // Encryption Keys (RSA-OAEP)
    pub public_key: Option<String>,
    pub encrypted_private_key: Option<String>,
    pub encrypted_backup_key: Option<String>,

    // Signing Keys (Ed25519)
    pub public_signing_key: Option<String>,
    pub encrypted_private_signing_key: Option<String>,
    pub encrypted_private_backup_signing_key: Option<String>,

    // Password Hashing
    pub argon_salt: Option<String>,
}
