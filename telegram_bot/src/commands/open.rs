use core::{
    error::Error as SpectreError, tip_context::TipContext, tip_owned_wallet::TipOwnedWallet,
};
use std::sync::Arc;

use spectre_wallet_keys::secret::Secret;
use teloxide::{Bot, types::Message};
use user::user::TipUser;

use crate::error::TelegramBotError;

pub async fn command_open(
    bot: Bot,
    _msg: &Message,
    tip_context: Arc<TipContext>,
    tip_user: &TipUser,
    password: String,
) -> Result<(), TelegramBotError> {
    // already opened
    if let Some(wallet) = tip_context.get_opened_owned_wallet(tip_user.wallet_identifier()) {
        tip_user
            .send_telegram_message(
                &bot,
                format!(
                    "Wallet Already Opened. Your wallet address: `{}`",
                    wallet.receive_address()
                ),
            )
            .await?;
        return Ok(());
    }

    let tip_wallet_result = TipOwnedWallet::open(
        tip_context.clone(),
        &Secret::from(password),
        tip_user.wallet_identifier(),
    )
    .await;

    let tip_wallet = match tip_wallet_result {
        Ok(t) => t,
        Err(SpectreError::WalletError(spectre_wallet_core::error::Error::WalletDecrypt(_))) => {
            tip_user
                .send_telegram_message(&bot, "Password is wrong. Please try again.")
                .await?;
            return Ok(());
        }
        Err(SpectreError::WalletError(spectre_wallet_core::error::Error::NoWalletInStorage(
            _message,
        ))) => {
            tip_user
                .send_telegram_message(&bot, "No wallet found. Please create one first.")
                .await?;
            return Ok(());
        }
        Err(error) => return Err(TelegramBotError::SpectreError(error)),
    };

    tip_user
        .send_telegram_message(
            &bot,
            format!(
                "Wallet Opened Successfully. Your wallet address: `{}`",
                tip_wallet.receive_address()
            ),
        )
        .await?;

    Ok(())
}
