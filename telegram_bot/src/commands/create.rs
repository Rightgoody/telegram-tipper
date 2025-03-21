use core::{tip_context::TipContext, tip_owned_wallet::TipOwnedWallet};
use std::sync::Arc;

use spectre_wallet_keys::secret::Secret;
use teloxide::{
    Bot,
    payloads::SendMessage,
    prelude::*,
    types::{Message, ParseMode},
};
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
        bot.send_message(tip_user.identifier(), "You already have a wallet.")
            .await?;
        return Ok(());
    }

    // password check
    if password.len() < 8 {
        bot.send_message(
            tip_user.identifier(),
            "Password must be at least 8 characters long.",
        )
        .await?;
        return Ok(());
    }

    let (tip_wallet, mnemonic) = TipOwnedWallet::create(
        tip_context.clone(),
        &Secret::from(password),
        tip_user.wallet_identifier(),
    )
    .await?;

    bot.send_message(tip_user.identifier(), format!("Wallet created\nMnemonic: `{}`\nPlease **save it securely**\\. You will need it to restore your wallet\\.\nReceive Address: `{}`", mnemonic.phrase_string(), tip_wallet.receive_address())).parse_mode(ParseMode::MarkdownV2).await?;

    return Ok(());
}
