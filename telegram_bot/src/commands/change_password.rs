use core::{error::Error as SpectreError, tip_context::TipContext};
use std::sync::Arc;

use spectre_wallet_keys::secret::Secret;
use teloxide::{Bot, types::Message};
use user::user::TipUser;

use crate::error::TelegramBotError;

pub async fn command_change_password(
    bot: Bot,
    _msg: &Message,
    tip_context: Arc<TipContext>,
    tip_user: &TipUser,
    old_password: String,
    new_password: String,
) -> Result<(), TelegramBotError> {
    if !tip_context.does_opened_owned_wallet_exists(tip_user.wallet_identifier()) {
        tip_user
            .send_telegram_message(
                &bot,
                "Error while changing the wallet password, wallet is not opened.",
            )
            .await?;

        return Ok(());
    }

    let tip_wallet = tip_context
        .get_opened_owned_wallet(tip_user.wallet_identifier())
        .ok_or(SpectreError::Custom("Wallet not found".to_owned()))?;

    // change secret
    match tip_wallet
        .change_secret(&Secret::from(old_password), &Secret::from(new_password))
        .await
    {
        Ok(_) => {
            tip_user
                .send_telegram_message(&bot, "Password changed successfully.")
                .await?;
            Ok(())
        }
        Err(SpectreError::WalletError(spectre_wallet_core::error::Error::WalletDecrypt(_))) => {
            tip_user
                .send_telegram_message(&bot, "Old password is incorrect.")
                .await?;
            Ok(())
        }
        Err(error) => Err(TelegramBotError::Custom(format!(
            "Error while changing the wallet password: {}",
            error
        ))),
    }
}
