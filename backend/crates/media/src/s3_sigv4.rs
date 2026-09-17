//! AWS Signature Version 4 query-string signing (presigned URLs) for
//! S3-compatible object stores, shared by private files and public media.
//!
//! Only `UNSIGNED-PAYLOAD` requests are signed; an optional
//! `x-amz-checksum-sha256` header can be bound into the signature so the store
//! rejects a body whose digest differs.

use base64::Engine;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use stocklink_shared::config::StorageConfig;
use stocklink_shared::errors::{AppError, AppResult};

type HmacSha256 = Hmac<Sha256>;

/// Everything that goes into one presigned request.
pub struct SigningInput<'a> {
    pub method: &'a str,
    pub host: &'a str,
    /// Already percent-encoded, starting with `/`.
    pub canonical_uri: &'a str,
    pub region: &'a str,
    pub access_key_id: &'a str,
    pub secret_access_key: &'a str,
    pub expires_secs: u64,
    pub timestamp: DateTime<Utc>,
    /// Base64 SHA-256 digest bound as a signed header, if any.
    pub checksum_sha256_b64: Option<&'a str>,
}

/// Presign a path-style URL (`{endpoint}/{bucket}/{key}`) valid for the
/// configured TTL (clamped to S3's 1 second – 7 day range).
pub fn presign_path_style(
    config: &StorageConfig,
    method: &str,
    object_key: &str,
    checksum_sha256_hex: Option<&str>,
) -> AppResult<String> {
    let host = endpoint_host(&config.endpoint_url)?;
    let canonical_uri = format!(
        "/{}/{}",
        encode_path(&config.bucket),
        encode_path(object_key)
    );
    let checksum_b64 = checksum_sha256_hex.map(checksum_header).transpose()?;
    let query = signed_query(&SigningInput {
        method,
        host: &host,
        canonical_uri: &canonical_uri,
        region: &config.region,
        access_key_id: &config.access_key_id,
        secret_access_key: &config.secret_access_key,
        expires_secs: config.presign_ttl.as_secs().clamp(1, 604_800),
        timestamp: Utc::now(),
        checksum_sha256_b64: checksum_b64.as_deref(),
    });
    Ok(format!(
        "{}{canonical_uri}?{query}",
        config.endpoint_url.trim_end_matches('/')
    ))
}

/// The complete presigned query string, ending in `X-Amz-Signature`.
pub fn signed_query(input: &SigningInput<'_>) -> String {
    let timestamp = input.timestamp.format("%Y%m%dT%H%M%SZ").to_string();
    let date = &timestamp[..8];
    let credential = format!(
        "{}/{}/{}/s3/aws4_request",
        input.access_key_id, date, input.region
    );
    let signed_headers = if input.checksum_sha256_b64.is_some() {
        "host;x-amz-checksum-sha256"
    } else {
        "host"
    };
    let canonical_query = format!(
        "X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential={}&X-Amz-Date={}&X-Amz-Expires={}&X-Amz-SignedHeaders={}",
        percent_encode(&credential),
        timestamp,
        input.expires_secs,
        percent_encode(signed_headers)
    );
    let canonical_headers = match input.checksum_sha256_b64 {
        Some(checksum) => format!("host:{}\nx-amz-checksum-sha256:{checksum}\n", input.host),
        None => format!("host:{}\n", input.host),
    };
    let canonical_request = format!(
        "{}\n{}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\nUNSIGNED-PAYLOAD",
        input.method, input.canonical_uri
    );
    let canonical_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
    let scope = format!("{date}/{}/s3/aws4_request", input.region);
    let string_to_sign = format!("AWS4-HMAC-SHA256\n{timestamp}\n{scope}\n{canonical_hash}");
    let key = signing_key(input.secret_access_key, date, input.region);
    let signature = hex::encode(hmac_bytes(&key, string_to_sign.as_bytes()));
    format!("{canonical_query}&X-Amz-Signature={signature}")
}

/// Hex SHA-256 digest → the base64 form S3 expects in `x-amz-checksum-sha256`.
pub fn checksum_header(checksum_sha256_hex: &str) -> AppResult<String> {
    let digest = hex::decode(checksum_sha256_hex).map_err(|_| AppError::Validation {
        field: "checksum_sha256".into(),
        message: "checksum must be a 64-character SHA-256 hex digest".into(),
    })?;
    Ok(base64::engine::general_purpose::STANDARD.encode(digest))
}

fn signing_key(secret: &str, date: &str, region: &str) -> Vec<u8> {
    let date_key = hmac_bytes(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let region_key = hmac_bytes(&date_key, region.as_bytes());
    let service_key = hmac_bytes(&region_key, b"s3");
    hmac_bytes(&service_key, b"aws4_request")
}

fn hmac_bytes(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts every key length");
    mac.update(message);
    mac.finalize().into_bytes().to_vec()
}

pub fn endpoint_host(endpoint: &str) -> AppResult<String> {
    let without_scheme = endpoint
        .split_once("://")
        .map(|(_, rest)| rest)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("storage endpoint has no scheme")))?;
    let host = without_scheme.split('/').next().unwrap_or_default();
    if host.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "storage endpoint has no host"
        )));
    }
    Ok(host.to_string())
}

pub fn encode_path(value: &str) -> String {
    value
        .split('/')
        .map(percent_encode)
        .collect::<Vec<_>>()
        .join("/")
}

pub fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AWS's published presigned-GET example ("Authenticating Requests: Using
    /// Query Parameters (AWS Signature Version 4)", Amazon S3 API Reference):
    /// GET examplebucket/test.txt, 20130524T000000Z, us-east-1, 86400 s.
    #[test]
    fn reproduces_the_aws_documented_presigned_get_signature() {
        let query = signed_query(&SigningInput {
            method: "GET",
            host: "examplebucket.s3.amazonaws.com",
            canonical_uri: "/test.txt",
            region: "us-east-1",
            access_key_id: "AKIAIOSFODNN7EXAMPLE",
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            expires_secs: 86_400,
            timestamp: DateTime::parse_from_rfc3339("2013-05-24T00:00:00Z")
                .expect("fixed timestamp")
                .with_timezone(&Utc),
            checksum_sha256_b64: None,
        });
        assert_eq!(
            query,
            "X-Amz-Algorithm=AWS4-HMAC-SHA256\
             &X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fus-east-1%2Fs3%2Faws4_request\
             &X-Amz-Date=20130524T000000Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host\
             &X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"
        );
    }

    #[test]
    fn encodes_object_paths_without_leaking_slashes_into_segments() {
        assert_eq!(
            encode_path("accounts/a/files/b c"),
            "accounts/a/files/b%20c"
        );
        assert_eq!(percent_encode("a+b"), "a%2Bb");
    }
}
