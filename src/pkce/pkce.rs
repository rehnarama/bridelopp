extern crate base64;
extern crate rand;
extern crate sha2;

use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sha2::{Digest, Sha256};

const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
    abcdefghijklmnopqrstuvwxyz\
    0123456789-.~_";

/// Generate a random code verifier.
///
/// # Arguments
///
/// * `length` - The desired length in bytes of the code verifier. This value should be between 43 and 128 or else the function will panic.
pub fn code_verifier(length: usize) -> Vec<u8> {
    assert!(
        (43..=128).contains(&length),
        "Code verifier length must be between 43 and 128 bytes"
    );

    let mut rng = thread_rng();

    (0..length)
        .map(|_| {
            let i = rng.gen_range(0..CHARS.len());
            CHARS[i]
        })
        .collect()
}

pub fn code_verifier_string(verifier: Vec<u8>) -> String {
    verifier.into_iter().map(|byte| byte as char).collect()
}

fn base64_url_encode(input: &[u8]) -> String {
    let b64 = base64::encode(input);
    b64.chars()
        .filter_map(|c| match c {
            '=' => None,
            '+' => Some('-'),
            '/' => Some('_'),
            x => Some(x),
        })
        .collect()
}

/// Generate a code challenge from a given code verifier with SHA256 and base64.
///
/// # Arguments
///
/// * `code_verifier` - The code verifier, such as the one generated by the [`code_verifier`] function.
pub fn code_challenge(code_verifier: &[u8]) -> String {
    let mut sha = Sha256::new();
    sha.update(code_verifier);
    let result = sha.finalize();
    base64_url_encode(&result[..])
}

pub fn random_state(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}
