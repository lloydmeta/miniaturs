use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use thiserror::Error;

type HmacSha1 = Hmac<Sha1>;

pub fn ensure_signature_is_valid_for_path_and_query(
    secret: &str,
    path_and_query_with_signature: &str,
    signature_to_check: &str,
) -> Result<(), SignatureError> {
    let signature_to_check_as_bytes = URL_SAFE
        .decode(signature_to_check.as_bytes())
        .map_err(|_| SignatureError::BadSignature)?;

    let path_without_signature = path_and_query_with_signature
        .strip_prefix(format!("/{signature_to_check}/").as_str())
        .unwrap_or(path_and_query_with_signature);

    let mut mac =
        HmacSha1::new_from_slice(secret.as_bytes()).map_err(|_| SignatureError::CouldNotUseKey)?;

    mac.update(path_without_signature.as_bytes());

    mac.verify_slice(&signature_to_check_as_bytes)
        .map_err(|_| SignatureError::BadSignature)
}

pub fn make_url_safe_base64_hash(secret: &str, message: &str) -> Result<String, SignatureError> {
    let mut mac =
        HmacSha1::new_from_slice(secret.as_bytes()).map_err(|_| SignatureError::CouldNotUseKey)?;

    mac.update(message.as_bytes());

    let result = mac.finalize();
    let result_bytes = result.into_bytes();

    Ok(URL_SAFE.encode(result_bytes))
}

#[derive(Debug, Error)]
pub enum SignatureError {
    #[error("Bad signature received; it did not pass the check.")]
    BadSignature,
    #[error("The secret key could not be used.")]
    CouldNotUseKey,
}

#[cfg(test)]
mod tests {
    use crate::signature::*;

    const SECRET: &'static str = "doyouwanttoknowasecretdoyoupromisenottotellwhoaohoh";
    const PATH: &'static str = "200x-100/https://beachape.com/images/octopress_with_container.png";
    // From https://www.liavaag.org/English/SHA-Generator/HMAC/
    const EXPECTED_SIGNED_BASE_64: &'static str = "Y/w4HN8q+yZPkR1N1SMJ9gDlCRk=";

    #[test]
    fn test_make_url_safe_base64_hash() -> Result<(), SignatureError> {
        let result = make_url_safe_base64_hash(SECRET, PATH)?;
        let expected = EXPECTED_SIGNED_BASE_64.replace("/", "_").replace("+", "-");
        assert_eq!(expected, result);
        Ok(())
    }
    #[test]
    fn test_ensure_signature_is_valid_for_path() -> Result<(), SignatureError> {
        let signature = EXPECTED_SIGNED_BASE_64.replace("/", "_").replace("+", "-");
        let path_with_signature = format!("/{signature}/{PATH}");
        let hashed = make_url_safe_base64_hash(SECRET, &path_with_signature)?;
        ensure_signature_is_valid_for_path_and_query(&SECRET, &path_with_signature, &hashed)
    }
}
