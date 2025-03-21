use std::{fmt::Debug, sync::Arc};

use futures::future::BoxFuture;
use teloxide::error_handlers::ErrorHandler;
use thiserror::Error;

use tracing::error;

#[derive(Debug, Error)]
pub enum TelegramBotError {
    #[error(transparent)]
    RequestError(#[from] teloxide::RequestError),
    #[error(transparent)]
    SpectreError(#[from] core::error::Error),
    #[error(transparent)]
    WalletError(#[from] spectre_wallet_core::error::Error),
}

pub struct LoggingErrorHandler {
    text: String,
}

impl LoggingErrorHandler {
    #[must_use]
    pub fn with_custom_text<T>(text: T) -> Arc<Self>
    where
        T: Into<String>,
    {
        Arc::new(Self { text: text.into() })
    }

    #[must_use]
    pub fn new() -> Arc<Self> {
        Self::with_custom_text("Error".to_owned())
    }
}

impl<E> ErrorHandler<E> for LoggingErrorHandler
where
    E: Debug,
{
    fn handle_error(self: Arc<Self>, error: E) -> BoxFuture<'static, ()> {
        error!("{text}: {:?}", error, text = self.text);
        Box::pin(async {})
    }
}
