use jsonwebtoken::{decode, decode_header, DecodingKey, Validation, Algorithm};
use serde::{Deserialize};
use reqwest::Client;

// JWKS response structure
#[derive(Deserialize)]
pub struct JwksResponse {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
pub struct Jwk {
    kty: String,
    kid: String,
    #[serde(rename = "use")]
    key_use: Option<String>,
    x: String,
    y: String,
    crv: String,
}

// Your JWT claims structure
#[derive(Debug, Deserialize)]
pub struct JwtClaims {
    sub: String,
    iss: String,
    email: String,
    #[serde(rename = "firstName")]
    first_name: String,
    #[serde(rename = "lastName")]
    last_name: String,
    exp: u64,
    iat: u64,
}

/// Decode and validate a JWT using the issuer's JWKS (JSON Web Key Set) and return its claims.
///
/// Fetches JWKS from `{base_url}/.well-known/jwks.json`, selects the JWK matching the token's `kid`,
/// constructs an EC decoding key, validates the token (ES256) with `iss == base_url`, and returns the
/// decoded `JwtClaims`. Errors are returned if the token header lacks a `kid`, the matching key is
/// not found in the JWKS, or verification/fetching fails.
///
/// # Examples
///
/// ```no_run
/// # use tokio;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let token = "eyJ..."; // a JWT string
/// let base_url = "https://issuer.example.com";
/// let claims = app::user_profile::auth::jwk::decode_jwt_safely(token, base_url).await?;
/// println!("sub: {}", claims.sub);
/// # Ok(()) }
/// # tokio::runtime::Runtime::new().unwrap().block_on(example()).unwrap();
/// ```
pub async fn decode_jwt_safely(token: &str, base_url: &str) -> Result<JwtClaims, Box<dyn std::error::Error>> {
    let header = decode_header(token)?;
    let kid = header.kid.ok_or("No kid in JWT header")?;

    let client = Client::new();
    let jwks_url = format!("{}/.well-known/jwks.json", base_url);
    let jwks: JwksResponse = client.get(&jwks_url).send().await?.json().await?;

    let jwk = jwks.keys.iter()
        .find(|k| k.kid == kid)
        .ok_or("Key not found in JWKS")?;

    let decoding_key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)?;

    let mut validation = Validation::new(Algorithm::ES256);
    validation.set_issuer(&[base_url]); // Validate issuer

    let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)?;

    Ok(token_data.claims)
}

// Generate consistent user ID
/// Derives a deterministic, short user identifier from JWT claims.
///
/// Combines the token's issuer and subject as `"{iss}:{sub}"`, hashes that with SHA-256,
/// encodes the digest using URL-safe base64 without padding, and returns the first 16
/// characters of the encoded string. Intended to produce a stable, non-reversible ID
/// that prevents cross-issuer collisions (different issuers with the same subject).
///
/// # Examples
///
/// ```
/// let claims = JwtClaims {
///     sub: "user-123".into(),
///     iss: "https://example.com".into(),
///     email: "user@example.com".into(),
///     first_name: "Alice".into(),
///     last_name: "Example".into(),
///     exp: 0,
///     iat: 0,
/// };
/// let id = generate_user_id(&claims);
/// assert_eq!(id.len(), 16);
/// ```
pub fn generate_user_id(claims: &JwtClaims) -> String {
    use sha2::{Sha256, Digest};
    use base64::prelude::*;

    // Combine sub and iss for security - prevents cross-issuer impersonation
    let combined_identity = format!("{}:{}", claims.iss, claims.sub);
    let digest = Sha256::digest(combined_identity.as_bytes());
    BASE64_URL_SAFE_NO_PAD.encode(digest)[..16].to_string()
}