use core::{
    tip_context::TipContext,
    utils::{estimate_fees, get_tx_explorer_url, try_parse_required_nonzero_spectre_as_sompi_u64},
};
use std::sync::Arc;

use spectre_wallet_core::{
    prelude::Address,
    tx::{Fees, PaymentOutputs},
};
use spectre_wallet_keys::secret::Secret;
use teloxide::{Bot, types::Message};
use user::user::TipUser;
use workflow_core::prelude::Abortable;

use crate::error::TelegramBotError;

pub async fn command_withdraw(
    bot: Bot,
    _msg: &Message,
    tip_context: Arc<TipContext>,
    tip_user: &TipUser,
    password: String,
    address: String,
    amount: String,
) -> Result<(), TelegramBotError> {
    let recipient_address = match Address::try_from(address.as_str()) {
        Ok(address) => address,
        Err(_) => {
            tip_user
                .send_telegram_message(
                    &bot,
                    "Error while withdrawing funds, invalid Spectre address",
                )
                .await?;

            return Ok(());
        }
    };

    let amount_sompi = try_parse_required_nonzero_spectre_as_sompi_u64(Some(amount))?;

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
            .send_telegram_message(&bot, "Error, wallet not initiated yet")
            .await?;

        return Ok(());
    }

    if !is_opened {
        tip_user
            .send_telegram_message(&bot, "Error, wallet not opened")
            .await?;

        return Ok(());
    }

    let tip_wallet = match tip_context.get_opened_owned_wallet(tip_user.wallet_identifier()) {
        Some(w) => w,
        None => {
            tip_user
                .send_telegram_message(&bot, "Unexpected error: wallet not opened")
                .await?;

            return Ok(());
        }
    };

    tip_user
        .send_telegram_message(&bot, "Withdrawing funds...")
        .await?;

    let wallet = tip_wallet.wallet();
    let account = wallet.account()?;

    let generator_summary_option = estimate_fees(
        &account,
        PaymentOutputs::from((recipient_address.clone(), amount_sompi)),
    )
    .await?;

    let amount_minus_gas_fee = match generator_summary_option.final_transaction_amount {
        Some(final_transaction_amount) => final_transaction_amount,
        None => {
            tip_user
                .send_telegram_message(&bot, "Error, While estimating the transaction fees, final_transaction_amount is None.")
                .await?;

            return Ok(());
        }
    };

    let abortable = Abortable::default();
    let wallet_secret = Secret::from(password);
    let outputs = PaymentOutputs::from((recipient_address.clone(), amount_minus_gas_fee));

    let (summary, hashes) = match account
        .send(
            outputs.into(),
            Fees::ReceiverPays(0),
            None,
            wallet_secret,
            None,
            &abortable,
            Some(Arc::new(
                move |ptx: &spectre_wallet_core::tx::PendingTransaction| {
                    println!("tx notifier: {:?}", ptx);
                },
            )),
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tip_user
                .send_telegram_message(&bot, format!("Error, Withdrawal failed: {}", e))
                .await?;

            return Ok(());
        }
    };

    let tx_id = hashes[0].to_string();

    tip_user
        .send_telegram_message(
            &bot,
            format!(
                "Withdrew to address `{}`: {}\nTxid: {}\nExplorer: {}",
                recipient_address,
                summary,
                tx_id,
                get_tx_explorer_url(&tx_id, *tip_context.network_id())
            ),
        )
        .await?;

    return Ok(());
}
