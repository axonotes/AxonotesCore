use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub email: String,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
}

pub fn decode_jwt_unsafe(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let key = DecodingKey::from_secret("dummy".as_ref());
    let mut validation = Validation::new(Algorithm::ES256);

    validation.validate_aud = false;
    validation.validate_nbf = false;
    validation.validate_exp = false;
    validation.insecure_disable_signature_validation();

    let token_data = decode::<Claims>(token, &key, &validation)?;
    Ok(token_data.claims)
}