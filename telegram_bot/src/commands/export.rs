use core::{tip_context::TipContext, tip_owned_wallet::TipOwnedWallet};
use std::sync::Arc;

use spectre_wallet_keys::secret::Secret;
use teloxide::{Bot, types::Message};
use user::user::TipUser;

use crate::error::TelegramBotError;

pub async fn command_export(
    bot: Bot,
    _msg: &Message,
    tip_context: Arc<TipContext>,
    tip_user: &TipUser,
    password: String,
) -> Result<(), TelegramBotError> {
    let wallet_exists = tip_context
        .local_store()?
        .exists(Some(tip_user.wallet_identifier()))
        .await?;

    if !wallet_exists {
        tip_user
            .send_telegram_message(&bot, "Error, wallet not found.")
            .await?;

        return Ok(());
    }

    let tip_wallet = TipOwnedWallet::open(
        tip_context.clone(),
        &Secret::from(password.clone()),
        tip_user.wallet_identifier(),
    )
    .await?;

    let (mnemonic, xpub) = tip_wallet
        .export_mnemonic_and_xpub(&Secret::from(password))
        .await?;

    if let Some(mnemonic) = mnemonic {
        tip_user
            .send_telegram_message(
                &bot,
                format!(
                    "Mnemonic Phrase: `{}`\nExtended Public Key (xpub): `{}`",
                    mnemonic.phrase(),
                    xpub
                ),
            )
            .await?;

        return Ok(());
    }

    Ok(())
}
