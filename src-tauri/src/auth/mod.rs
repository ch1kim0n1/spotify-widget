mod secret_store;

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex,
    time::timeout,
};
use url::Url;

pub use secret_store::{NativeSecretStore, SecretStore};

use crate::error::{AppError, AppResult};

const AUTHORIZE_URL: &str = "https://accounts.spotify.com/authorize";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const CALLBACK_PATH: &str = "/callback";
const SCOPES: &str =
    "user-read-playback-state user-read-currently-playing user-modify-playback-state";

#[derive(Debug, Clone)]
pub struct SpotifyConfig {
    pub client_id: String,
}

impl SpotifyConfig {
    pub fn from_environment() -> Option<Self> {
        option_env!("SPOTIFY_CLIENT_ID")
            .map(str::to_owned)
            .or_else(|| std::env::var("SPOTIFY_CLIENT_ID").ok())
            .filter(|value| !value.trim().is_empty())
            .map(|client_id| Self { client_id })
    }
}

#[derive(Debug, Clone)]
struct AccessToken {
    value: String,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Debug)]
struct CallbackResult {
    code: String,
    redirect_uri: String,
}

pub struct AuthManager {
    config: Option<SpotifyConfig>,
    client: Client,
    secrets: Arc<dyn SecretStore>,
    access_token: Mutex<Option<AccessToken>>,
    refresh_lock: Mutex<()>,
}

impl AuthManager {
    pub fn new(config: Option<SpotifyConfig>, secrets: Arc<dyn SecretStore>) -> AppResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent(format!(
                "SpotifyCompanionWidget/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self {
            config,
            client,
            secrets,
            access_token: Mutex::new(None),
            refresh_lock: Mutex::new(()),
        })
    }

    pub fn is_configured(&self) -> bool {
        self.config.is_some()
    }

    pub async fn has_credentials(&self) -> AppResult<bool> {
        Ok(self.secrets.get_refresh_token().await?.is_some())
    }

    pub async fn begin_authorization(&self) -> AppResult<()> {
        let config = self
            .config
            .as_ref()
            .ok_or(AppError::ConfigurationRequired)?;
        let verifier = random_urlsafe(64);
        let state = random_urlsafe(32);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .map_err(|error| AppError::Authorization(error.to_string()))?;
        let port = listener
            .local_addr()
            .map_err(|error| AppError::Authorization(error.to_string()))?
            .port();
        let redirect_uri = format!("http://127.0.0.1:{port}{CALLBACK_PATH}");

        let mut authorize_url = Url::parse(AUTHORIZE_URL)
            .map_err(|error| AppError::Authorization(error.to_string()))?;
        authorize_url
            .query_pairs_mut()
            .append_pair("client_id", &config.client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("scope", SCOPES)
            .append_pair("code_challenge_method", "S256")
            .append_pair("code_challenge", &challenge)
            .append_pair("state", &state);

        webbrowser::open(authorize_url.as_str())
            .map_err(|error| AppError::Authorization(error.to_string()))?;

        let callback = timeout(
            Duration::from_secs(180),
            receive_callback(listener, state, redirect_uri),
        )
        .await
        .map_err(|_| AppError::Authorization("authorization timed out".into()))??;

        let response = self
            .client
            .post(TOKEN_URL)
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("grant_type", "authorization_code"),
                ("code", callback.code.as_str()),
                ("redirect_uri", callback.redirect_uri.as_str()),
                ("code_verifier", verifier.as_str()),
            ])
            .send()
            .await?;
        let tokens = parse_token_response(response).await?;
        self.accept_tokens(tokens).await
    }

    pub async fn access_token(&self) -> AppResult<String> {
        {
            let token = self.access_token.lock().await;
            if let Some(token) = token.as_ref()
                && token.expires_at > Instant::now() + Duration::from_secs(60)
            {
                return Ok(token.value.clone());
            }
        }
        self.refresh_access_token().await
    }

    pub async fn invalidate_access_token(&self) {
        *self.access_token.lock().await = None;
    }

    pub async fn disconnect(&self) -> AppResult<()> {
        self.invalidate_access_token().await;
        self.secrets.delete_refresh_token().await
    }

    async fn refresh_access_token(&self) -> AppResult<String> {
        let _refresh_guard = self.refresh_lock.lock().await;
        {
            let token = self.access_token.lock().await;
            if let Some(token) = token.as_ref()
                && token.expires_at > Instant::now() + Duration::from_secs(60)
            {
                return Ok(token.value.clone());
            }
        }

        let config = self
            .config
            .as_ref()
            .ok_or(AppError::ConfigurationRequired)?;
        let refresh_token = self
            .secrets
            .get_refresh_token()
            .await?
            .ok_or(AppError::AuthenticationRequired)?;
        let response = self
            .client
            .post(TOKEN_URL)
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token.as_str()),
            ])
            .send()
            .await?;

        if response.status() == StatusCode::BAD_REQUEST {
            self.secrets.delete_refresh_token().await?;
            return Err(AppError::AuthenticationRequired);
        }

        let tokens = parse_token_response(response).await?;
        let value = tokens.access_token.clone();
        self.accept_tokens(tokens).await?;
        Ok(value)
    }

    async fn accept_tokens(&self, tokens: TokenResponse) -> AppResult<()> {
        if let Some(refresh_token) = tokens.refresh_token.as_deref() {
            self.secrets.put_refresh_token(refresh_token).await?;
        }
        let expires_in = Duration::from_secs(tokens.expires_in.max(60));
        *self.access_token.lock().await = Some(AccessToken {
            value: tokens.access_token,
            expires_at: Instant::now() + expires_in,
        });
        Ok(())
    }
}

async fn parse_token_response(response: reqwest::Response) -> AppResult<TokenResponse> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Authorization(format!(
            "token endpoint returned {status}: {}",
            truncate(&body, 160)
        )));
    }
    response
        .json()
        .await
        .map_err(|error| AppError::Authorization(error.to_string()))
}

async fn receive_callback(
    listener: TcpListener,
    expected_state: String,
    redirect_uri: String,
) -> AppResult<CallbackResult> {
    let (mut stream, peer) = listener
        .accept()
        .await
        .map_err(|error| AppError::Authorization(error.to_string()))?;
    if !peer.ip().is_loopback() {
        return Err(AppError::Authorization(
            "callback did not originate from loopback".into(),
        ));
    }

    let mut buffer = [0_u8; 8192];
    let read = stream
        .read(&mut buffer)
        .await
        .map_err(|error| AppError::Authorization(error.to_string()))?;
    let request = std::str::from_utf8(&buffer[..read])
        .map_err(|_| AppError::Authorization("callback request was not valid UTF-8".into()))?;
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| AppError::Authorization("callback request line was invalid".into()))?;
    match parse_callback_target(target, &expected_state) {
        Ok(code) => {
            write_callback_response(&mut stream, true).await;
            Ok(CallbackResult { code, redirect_uri })
        }
        Err(error) => {
            write_callback_response(&mut stream, false).await;
            Err(error)
        }
    }
}

fn parse_callback_target(target: &str, expected_state: &str) -> AppResult<String> {
    let callback_url = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|error| AppError::Authorization(error.to_string()))?;
    if callback_url.path() != CALLBACK_PATH {
        return Err(AppError::Authorization("unexpected callback path".into()));
    }
    let query = callback_url.query_pairs().collect::<Vec<_>>();
    let states = query
        .iter()
        .filter(|(name, _)| name == "state")
        .map(|(_, value)| value.as_ref())
        .collect::<Vec<_>>();
    if states.as_slice() != [expected_state] {
        return Err(AppError::Authorization("OAuth state did not match".into()));
    }
    if let Some((_, error)) = query.iter().find(|(name, _)| name == "error") {
        return Err(AppError::Authorization(error.to_string()));
    }
    let codes = query
        .iter()
        .filter(|(name, _)| name == "code")
        .map(|(_, value)| value.to_string())
        .collect::<Vec<_>>();
    if codes.len() != 1 || codes[0].is_empty() {
        return Err(AppError::Authorization(
            "authorization code was missing or ambiguous".into(),
        ));
    }
    Ok(codes[0].clone())
}

async fn write_callback_response(stream: &mut tokio::net::TcpStream, success: bool) {
    let (status, title, message) = if success {
        (
            "200 OK",
            "Spotify connected",
            "You can close this tab and return to Spotify Companion Widget.",
        )
    } else {
        (
            "400 Bad Request",
            "Connection failed",
            "Return to Spotify Companion Widget and try again.",
        )
    };
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>{title}</title><style>body{{font:16px system-ui;background:#111411;color:#eef6f0;display:grid;place-items:center;height:100vh;margin:0}}main{{max-width:32rem;padding:2rem}}h1{{color:#63dd83}}</style><main><h1>{title}</h1><p>{message}</p></main>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

fn random_urlsafe(bytes: usize) -> String {
    let random = (0..bytes).map(|_| rand::random::<u8>()).collect::<Vec<_>>();
    URL_SAFE_NO_PAD.encode(random)
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_pkce_values_are_url_safe() {
        let value = random_urlsafe(64);
        assert!(value.len() >= 80);
        assert!(!value.contains(['+', '/', '=']));
    }

    #[test]
    fn callback_requires_exact_state_and_single_code() {
        assert_eq!(
            parse_callback_target("/callback?code=abc&state=expected", "expected")
                .expect("valid callback"),
            "abc"
        );
        assert!(parse_callback_target("/callback?code=abc&state=wrong", "expected").is_err());
        assert!(
            parse_callback_target(
                "/callback?code=abc&code=duplicate&state=expected",
                "expected"
            )
            .is_err()
        );
        assert!(parse_callback_target("/other?code=abc&state=expected", "expected").is_err());
    }
}
