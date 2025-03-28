use core::{tip_context::TipContext, tip_owned_wallet::TipOwnedWallet};
use std::sync::Arc;

use spectre_wallet_core::prelude::{Language, Mnemonic};
use spectre_wallet_keys::secret::Secret;
use teloxide::{Bot, types::Message};
use user::user::TipUser;

use crate::error::TelegramBotError;

pub async fn command_restore(
    bot: Bot,
    _msg: &Message,
    tip_context: Arc<TipContext>,
    tip_user: &TipUser,
    mnemonic_phrase: String,
    password: String,
) -> Result<(), TelegramBotError> {
    let mnemonic = match Mnemonic::new(mnemonic_phrase.trim(), Language::English) {
        Ok(mnemonic) => {
            // is a valid BIP32 mnemonic (12 or 24 words)
            let word_count = mnemonic.phrase().split_whitespace().count();
            if word_count != 12 && word_count != 24 {
                tip_user
                    .send_telegram_message(
                        &bot,
                        "Error while restoring the wallet, mnemonic must be 12 or 24 words"
                            .to_string(),
                    )
                    .await?;

                return Ok(());
            }
            mnemonic
        }
        Err(_) => {
            tip_user
                .send_telegram_message(
                    &bot,
                    "Error while restoring the wallet, invalid mnemonic phrase".to_string(),
                )
                .await?;
            return Ok(());
        }
    };

    let recovered_tip_wallet_result = TipOwnedWallet::restore(
        tip_context.clone(),
        &Secret::from(password),
        mnemonic,
        tip_user.wallet_identifier(),
    )
    .await?;

    tip_user
        .send_telegram_message(
            &bot,
            format!(
                "Wallet Restored Successfully, receive address: {}",
                recovered_tip_wallet_result.receive_address().to_string(),
            ),
        )
        .await?;

    return Ok(());
}
