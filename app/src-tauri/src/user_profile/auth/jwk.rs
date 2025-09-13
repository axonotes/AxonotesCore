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
pub fn generate_user_id(claims: &JwtClaims) -> String {
    use sha2::{Sha256, Digest};
    use base64::prelude::*;

    // Combine sub and iss for security - prevents cross-issuer impersonation
    let combined_identity = format!("{}:{}", claims.iss, claims.sub);
    let digest = Sha256::digest(combined_identity.as_bytes());
    BASE64_URL_SAFE_NO_PAD.encode(digest)[..16].to_string()
}