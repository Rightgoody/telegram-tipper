use core::tip_context::TipContext;
use std::sync::Arc;

use teloxide::{Bot, types::Message};
use user::user::TipUser;

use crate::error::TelegramBotError;

pub async fn command_destroy(
    bot: Bot,
    _msg: &Message,
    tip_context: Arc<TipContext>,
    tip_user: &TipUser,
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

    if !is_initiated {
        tip_user
            .send_telegram_message(
                &bot,
                "Error, the wallet is not initiated, cannot destroy a non-existing wallet.",
            )
            .await?;

        return Ok(());
    }

    // @TODO(izio/tg) - discussion flow to confirm destruction
    tip_user
        .send_telegram_message(&bot, "Are you sure you want to destroy your wallet?")
        .await?;

    return Ok(());
}
