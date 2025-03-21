use core::{
    tip_context::TipContext, tip_transition_wallet::TipTransitionWallet, utils::estimate_fees,
};
use std::sync::Arc;

use futures::future::join_all;
use spectre_wallet_core::tx::{Fees, PaymentOutputs};
use spectre_wallet_keys::secret::Secret;
use teloxide::{Bot, types::Message};
use user::user::TipUser;
use workflow_core::prelude::Abortable;

use crate::error::TelegramBotError;

pub async fn command_claim(
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

    if !is_opened && !is_initiated {
        tip_user
            .send_telegram_message(&bot, "Wallet is not opened or initiated.")
            .await?;
        return Ok(());
    }

    let tip_wallet = match tip_context.get_opened_owned_wallet(tip_user.wallet_identifier()) {
        Some(w) => w,
        None => {
            tip_user
                .send_telegram_message(&bot, "Unexpected error while claiming funds.")
                .await?;

            return Ok(());
        }
    };

    tip_user
        .send_telegram_message(&bot, "Claiming funds...")
        .await?;

    let wallet = tip_wallet.wallet();
    let owner_receive_address = wallet.account().unwrap().receive_address().unwrap();

    let transition_wallets = tip_context
        .transition_wallet_metadata_store
        .find_transition_wallet_metadata_by_target_identifier(tip_user.wallet_identifier())
        .await?;

    // check if there are any pending balances in transition wallets
    let pending_transition_balance = join_all(transition_wallets.iter().map(|metadata| async {
        let secret = Secret::from(metadata.secret.clone());
        let transition_wallet = TipTransitionWallet::open(
            tip_context.clone(),
            &secret,
            &metadata.initiator_identifier,
            &metadata.target_identifier,
        )
        .await;

        let balance: u64 = match transition_wallet {
            Ok(tw) => {
                let account_result = tw.wallet().account();
                let mut b = 0;
                if let Ok(account) = account_result {
                    if let Some(balance) = account.balance() {
                        b = balance.mature
                    }
                }

                let _ = tw.wallet().stop().await;

                b
            }
            Err(e) => {
                println!("warning: {:?}", e);

                0_u64
            }
        };

        balance
    }))
    .await
    .into_iter()
    .reduce(|a, b| a + b);

    if pending_transition_balance.unwrap_or(0) == 0 {
        tip_user
            .send_telegram_message(&bot, "No coins stored in the transition wallets, aborting.")
            .await?;
        return Ok(());
    }

    join_all(transition_wallets.iter().map(|metadata| async {
        let secret = Secret::from(metadata.secret.clone());
        let transition_wallet = TipTransitionWallet::open(
            tip_context.clone(),
            &secret,
            &metadata.initiator_identifier,
            &metadata.target_identifier,
        )
        .await;

        match transition_wallet {
            Ok(tw) => {
                let account_result = tw.wallet().account();
                if let Ok(account) = account_result {
                    if let Some(balance) = account.balance() {
                        let receive_address = owner_receive_address.clone();

                        println!(
                            "sending {} SPR from {} to {}",
                            balance.mature,
                            account.receive_address().unwrap().address_to_string(),
                            owner_receive_address.address_to_string()
                        );

                        let address = receive_address;

                        let amount_sompi = balance.mature;

                        let outputs = PaymentOutputs::from((address.clone(), amount_sompi));

                        let generator_summary_option =
                            estimate_fees(&account, outputs.clone()).await?;

                        let amount_minus_gas_fee =
                            match generator_summary_option.final_transaction_amount {
                                Some(final_transaction_amount) => final_transaction_amount,
                                None => {
                                    tip_user.send_telegram_message(&bot, "Error while estimating the transaction fees, final_transaction_amount is None.").await?;

                                    return Ok(());
                                },
                            };

                        println!(
                            "amount sompi {}, with gas fee {}",
                            amount_sompi, amount_minus_gas_fee
                        );

                        let outputs = PaymentOutputs::from((address, amount_minus_gas_fee));
                        let abortable = Abortable::default();

                        let (summary, hashes) = account
                            .send(
                                outputs.into(),
                                Fees::ReceiverPays(0),
                                None,
                                secret,
                                None,
                                &abortable,
                                Some(Arc::new(
                                    move |ptx: &spectre_wallet_core::tx::PendingTransaction| {
                                        println!("tx notifier: {:?}", ptx);
                                    },
                                )),
                            )
                            .await
                            .unwrap();

                        println!("summary {:?}, hashes: {:?}", summary, hashes);

                        tip_user.send_telegram_message(&bot, format!("Successfully claimed funds from transition wallets. summary {:?}\n hashes: {:?}", summary, hashes)).await?;
                    }
                }

                tw.wallet().stop().await?;
            }
            Err(e) => {
                println!("warning: {:?}", e);
            }
        };

        Ok::<(), TelegramBotError>(())
    }))
    .await;

    Ok(())
}
