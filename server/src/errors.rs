use std::fmt;

#[derive(Debug)]
pub enum AuthError {
    UserNotFound,
    NoPublicKey,
    InvalidKeyFormat(String),
    InvalidSignature,
    DecodeError(String),
}

impl fmt::Display for AuthError {
    /// Formats an `AuthError` into a concise, human-readable message.
    ///
    /// Each enum variant is rendered as a short description (e.g. `User not found`,
    /// `Invalid signature`, or `Invalid key format: <msg>`).
    ///
    /// # Examples
    ///
    /// ```
    /// let s = format!("{}", AuthError::UserNotFound);
    /// assert_eq!(s, "User not found");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::NoPublicKey => write!(f, "User has no public signing key"),
            AuthError::InvalidKeyFormat(msg) => write!(f, "Invalid key format: {}", msg),
            AuthError::InvalidSignature => write!(f, "Invalid signature"),
            AuthError::DecodeError(msg) => write!(f, "Decode error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

#[derive(Debug)]
pub enum UserError {
    AlreadyInitialized,
    NotInitialized,
    InvalidData(String),
}

impl fmt::Display for UserError {
    /// Formats a `UserError` as a human-readable message.
    ///
    /// Produces the following strings for each variant:
    /// - `AlreadyInitialized` -> `"User keys already initialized"`
    /// - `NotInitialized` -> `"User keys not initialized"`
    /// - `InvalidData(msg)` -> `"Invalid data: {msg}"`
    ///
    /// # Examples
    ///
    /// ```
    /// use server::errors::UserError;
    ///
    /// assert_eq!(format!("{}", UserError::AlreadyInitialized), "User keys already initialized");
    /// assert_eq!(format!("{}", UserError::NotInitialized), "User keys not initialized");
    /// assert_eq!(format!("{}", UserError::InvalidData("bad".into())), "Invalid data: bad");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserError::AlreadyInitialized => write!(f, "User keys already initialized"),
            UserError::NotInitialized => write!(f, "User keys not initialized"),
            UserError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl std::error::Error for UserError {}