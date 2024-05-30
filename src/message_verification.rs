use axum::{
    body::Body,
    http::{header::HeaderMap, HeaderValue},
};
use hmac::{digest::MacError, Hmac, Mac};
use sha2::Sha256;
use std::fmt::Display;
type HmacSha256 = Hmac<Sha256>;
use hex_literal::hex;
pub fn verify_message(
    headers: &HeaderMap,
    body: &String,
    secret: &String,
) -> Result<(), HmacVerificationError> {
    let message = get_hmac_message(headers, body)?;
    let signature = match headers.get("twitch-eventsub-message-signature") {
        Some(x) => signature_to_hex(&x)?,
        None => {
            return Err(HmacVerificationError::MissingHeader(String::from(
                "twitch-eventsub-message-signature",
            )))
        }
    };

    let mut mac =
        HmacSha256::new_from_slice(secret[..].as_bytes()).expect("HMAC can take key of any size");
    mac.update(message[..].as_bytes());
    match mac.verify_slice(&signature) {
        Ok(()) => Ok(()),
        Err(err) => Err(HmacVerificationError::MacError(err)),
    }
}

fn get_hmac_message(headers: &HeaderMap, body: &String) -> Result<String, HmacVerificationError> {
    let message_id = match headers.get("twitch-eventsub-message-id") {
        Some(x) => x.to_str().unwrap(),
        None => {
            return Err(HmacVerificationError::MissingHeader(String::from(
                "twitch-eventsub-message-id",
            )))
        }
    };
    let message_timestamp = match headers.get("twitch-eventsub-message-timestamp") {
        Some(x) => x.to_str().unwrap(),
        None => {
            return Err(HmacVerificationError::MissingHeader(String::from(
                "twitch-eventsub-message-timestamp",
            )))
        }
    };
    Ok(format!(
        "{}{}{}",
        message_id,
        message_timestamp,
        body.clone()
    ))
}

fn signature_to_hex(signature: &HeaderValue) -> Result<Vec<u8>, HmacVerificationError> {
    let signature = signature.to_str().unwrap();

    if signature.len() < 7 {
        return Err(HmacVerificationError::InvalidSignature);
    }

    let signature = &signature[7..];
    // Ensure the signature string length is even
    if signature.len() % 2 != 0 {
        return Err(HmacVerificationError::InvalidSignature);
    }

    // Convert the signature string into a Vec<u8>
    let bytes: Result<Vec<u8>, _> = (0..signature.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&signature[i..i + 2], 16)
                .map_err(|_| Err::<u8, _>(HmacVerificationError::InvalidSignature))
        })
        .collect();

    match bytes {
        Ok(x) => Ok(x),
        Err(_) => Err(HmacVerificationError::InvalidSignature),
    }
}

#[derive(Debug)]
pub enum HmacVerificationError {
    MissingHeader(String),
    MacError(MacError),
    InvalidSignature,
}

impl Display for HmacVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HmacVerificationError::MissingHeader(missing_header) => {
                write!(f, "header {} missing", missing_header)
            }
            HmacVerificationError::MacError(err) => {
                write!(f, "macerror: {}", err)
            }
            HmacVerificationError::InvalidSignature => {
                write!(f, "invalid signature in header")
            }
        }
    }
}

impl std::error::Error for HmacVerificationError {}
