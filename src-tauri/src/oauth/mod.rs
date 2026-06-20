// Desktop OAuth — loopback redirect + PKCE — for the Google Health API.
// Ported from the reference gh-auth.mjs / health-steps.mjs. No browser-side
// secret handling: the refresh token is exchanged here and stored encrypted.
use std::io::{Read, Write};
use std::net::TcpListener;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64URL, Engine};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("authorization denied: {0}")]
    Denied(String),
    #[error("state mismatch — possible CSRF, aborting")]
    StateMismatch,
    #[error("no authorization code returned")]
    NoCode,
    #[error("token endpoint error: {0}")]
    Token(String),
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

fn b64url(bytes: &[u8]) -> String {
    B64URL.encode(bytes)
}

pub struct AuthFlow {
    pub listener: TcpListener,
    pub redirect_uri: String,
    pub auth_url: String,
    pub verifier: String,
    pub state: String,
}

/// Bind a loopback listener on a random port and build the PKCE consent URL.
pub fn start_flow(client_id: &str, scope: &str) -> Result<AuthFlow, OAuthError> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");

    let mut verifier_bytes = [0u8; 32];
    let mut state_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut verifier_bytes);
    rand::thread_rng().fill_bytes(&mut state_bytes);
    let verifier = b64url(&verifier_bytes);
    let state = b64url(&state_bytes);
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = b64url(&digest);

    let auth_url = url::Url::parse_with_params(
        AUTH_ENDPOINT,
        &[
            ("client_id", client_id),
            ("redirect_uri", &redirect_uri),
            ("response_type", "code"),
            ("scope", scope),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("access_type", "offline"),
            // `select_account` always shows the account chooser (so the user can
            // pick a different account — e.g. one that's linked to Google Health);
            // `consent` guarantees a refresh token is returned.
            ("prompt", "select_account consent"),
            ("state", &state),
        ],
    )
    .map_err(|e| OAuthError::Token(e.to_string()))?
    .to_string();

    Ok(AuthFlow {
        listener,
        redirect_uri,
        auth_url,
        verifier,
        state,
    })
}

/// Block until Google redirects back with `?code=…`. Run inside spawn_blocking.
pub fn wait_for_code(listener: TcpListener, expected_state: &str) -> Result<String, OAuthError> {
    let (mut stream, _) = listener.accept()?;
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let parsed = url::Url::parse(&format!("http://127.0.0.1{path}"))
        .map_err(|e| OAuthError::Token(e.to_string()))?;

    let body = "<!doctype html><meta charset=utf-8><body style=\"font:15px -apple-system,system-ui;padding:48px;text-align:center;color:#1d1d1f\"><h2 style=\"font-weight:600\">Stepwise is connected.</h2><p style=\"color:#86868b\">You can close this tab and return to the app.</p>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());

    let params: std::collections::HashMap<String, String> =
        parsed.query_pairs().into_owned().collect();
    if let Some(err) = params.get("error") {
        return Err(OAuthError::Denied(err.clone()));
    }
    if params.get("state").map(String::as_str) != Some(expected_state) {
        return Err(OAuthError::StateMismatch);
    }
    params.get("code").cloned().ok_or(OAuthError::NoCode)
}

pub async fn exchange_code(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, OAuthError> {
    let resp = http
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("code_verifier", verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?;
    parse_token(resp).await
}

pub async fn refresh(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<TokenResponse, OAuthError> {
    let resp = http
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;
    parse_token(resp).await
}

async fn parse_token(resp: reqwest::Response) -> Result<TokenResponse, OAuthError> {
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(OAuthError::Token(text));
    }
    serde_json::from_str::<TokenResponse>(&text).map_err(|e| OAuthError::Token(e.to_string()))
}
