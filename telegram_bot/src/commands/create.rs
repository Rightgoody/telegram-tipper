use core::{tip_context::TipContext, tip_owned_wallet::TipOwnedWallet};
use std::sync::Arc;

use spectre_wallet_keys::secret::Secret;
use teloxide::{Bot, types::Message};
use user::user::TipUser;

use crate::error::TelegramBotError;

pub async fn command_create(
    bot: Bot,
    _msg: &Message,
    tip_context: Arc<TipContext>,
    tip_user: &TipUser,
    password: String,
) -> Result<(), TelegramBotError> {
    let is_opened = tip_context.does_opened_owned_wallet_exists(tip_user.wallet_identifier());
    let is_initiated = match is_opened {
        true => true,
        false => {
            tip_context
                .local_store()?
                .exists(Some(tip_user.wallet_identifier()))
                .await?
        }
    };
    if is_initiated {
        tip_user
            .send_telegram_message(&bot, "You already have a wallet.")
            .await?;
        return Ok(());
    }

    // password check
    if password.len() < 8 {
        tip_user
            .send_telegram_message(&bot, "Password must be at least 8 characters long.")
            .await?;
        return Ok(());
    }

    let (tip_wallet, mnemonic) = TipOwnedWallet::create(
        tip_context.clone(),
        &Secret::from(password),
        tip_user.wallet_identifier(),
    )
    .await?;

    tip_user
            .send_telegram_message(&bot, format!("Wallet created\nMnemonic: `{}`\nPlease **save it securely** and **never** change your username, as username is used for identification of ownership. You will need it to restore your wallet.\nReceive Address: `{}`", mnemonic.phrase_string(), tip_wallet.receive_address())).await?;

    return Ok(());
}
