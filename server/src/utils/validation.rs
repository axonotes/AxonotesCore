/// Helper functions for input validation
pub fn validate_base64(input: &str) -> Result<(), String> {
    use base64::engine::general_purpose;
    use base64::Engine;

    general_purpose::STANDARD
        .decode(input)
        .map_err(|_| "Invalid base64 format".to_string())?;
    Ok(())
}

pub fn validate_key_length(
    key: &str,
    expected_length: usize,
) -> Result<(), String> {
    use base64::engine::general_purpose;
    use base64::Engine;

    let decoded = general_purpose::STANDARD
        .decode(key)
        .map_err(|_| "Invalid base64 format".to_string())?;

    if decoded.len() != expected_length {
        return Err(format!(
            "Key length mismatch: expected {} bytes, got {}",
            expected_length,
            decoded.len()
        ));
    }
    Ok(())
}
