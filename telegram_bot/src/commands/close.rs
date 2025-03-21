use core::tip_context::TipContext;
use std::sync::Arc;

use teloxide::{Bot, types::Message};
use user::user::TipUser;

use crate::error::TelegramBotError;

pub async fn command_close(
    bot: Bot,
    _msg: &Message,
    tip_context: Arc<TipContext>,
    tip_user: &TipUser,
) -> Result<(), TelegramBotError> {
    let is_opened = tip_context.does_opened_owned_wallet_exists(tip_user.wallet_identifier());

    if is_opened {
        let tip_wallet_result =
            tip_context.remove_opened_owned_wallet(tip_user.wallet_identifier());

        if let Some(tip_wallet) = tip_wallet_result {
            tip_wallet.wallet().stop().await?;
            tip_wallet.wallet().close().await?;
        }
    }

    tip_user
        .send_telegram_message(&bot, "Your wallet has been successfully closed.")
        .await?;

    Ok(())
}
