use async_trait::async_trait;
use keyring::{Entry, Error as KeyringError};

use crate::error::{AppError, AppResult};

const SERVICE: &str = "com.vladislav.spotify-companion-widget";
const REFRESH_TOKEN_ACCOUNT: &str = "spotify-refresh-token";

#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn put_refresh_token(&self, token: &str) -> AppResult<()>;
    async fn get_refresh_token(&self) -> AppResult<Option<String>>;
    async fn delete_refresh_token(&self) -> AppResult<()>;
}

#[derive(Debug, Default)]
pub struct NativeSecretStore;

impl NativeSecretStore {
    fn entry() -> AppResult<Entry> {
        Entry::new(SERVICE, REFRESH_TOKEN_ACCOUNT)
            .map_err(|error| AppError::SecureStore(error.to_string()))
    }
}

#[async_trait]
impl SecretStore for NativeSecretStore {
    async fn put_refresh_token(&self, token: &str) -> AppResult<()> {
        let token = token.to_owned();
        tokio::task::spawn_blocking(move || {
            Self::entry()?
                .set_password(&token)
                .map_err(|error| AppError::SecureStore(error.to_string()))
        })
        .await
        .map_err(|error| AppError::SecureStore(error.to_string()))?
    }

    async fn get_refresh_token(&self) -> AppResult<Option<String>> {
        tokio::task::spawn_blocking(move || match Self::entry()?.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(AppError::SecureStore(error.to_string())),
        })
        .await
        .map_err(|error| AppError::SecureStore(error.to_string()))?
    }

    async fn delete_refresh_token(&self) -> AppResult<()> {
        tokio::task::spawn_blocking(move || match Self::entry()?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(AppError::SecureStore(error.to_string())),
        })
        .await
        .map_err(|error| AppError::SecureStore(error.to_string()))?
    }
}
