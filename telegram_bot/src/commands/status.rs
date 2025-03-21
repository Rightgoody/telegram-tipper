use core::{tip_context::TipContext, tip_transition_wallet::TipTransitionWallet};
use std::sync::Arc;

use futures::future::join_all;
use spectre_wallet_core::utils::sompi_to_spectre_string_with_suffix;
use spectre_wallet_keys::secret::Secret;
use teloxide::{Bot, types::Message};
use user::user::TipUser;

use crate::error::TelegramBotError;

pub async fn command_status(
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
        tip_user.send_telegram_message(&bot, "Wallet Status:\n\nThe wallet has not been created yet. Use the `create` command to create a wallet.").await?;
    }

    if !is_opened {
        tip_user.send_telegram_message(&bot, "Wallet Status:\n\nThe wallet is not opened. Use the `open` command to open the wallet and display its balance.").await?;
    }

    let owned_wallet = tip_context
        .get_opened_owned_wallet(&tip_user.wallet_identifier())
        .unwrap();

    let account = owned_wallet.wallet().account().unwrap();

    let balance = account.balance().unwrap_or_default();

    let transition_wallets = tip_context
        .transition_wallet_metadata_store
        .find_transition_wallet_metadata_by_target_identifier(&tip_user.wallet_identifier())
        .await?;

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

    let network_type = tip_context.network_id();
    let balance_formatted = sompi_to_spectre_string_with_suffix(balance.mature, &network_type);
    let pending_balance_formatted =
        sompi_to_spectre_string_with_suffix(balance.pending, &network_type);

    let pending_transition_balance_formatted =
        sompi_to_spectre_string_with_suffix(pending_transition_balance.unwrap_or(0), &network_type);

    tip_user.send_telegram_message(
        &bot,
        format!(
            "Your wallet balance is {} \n Pending balance is {} \n UTXO count is {} \n Pending UTXO count is {} \n Balance to be claimed is {}",
            balance_formatted,
            pending_balance_formatted,
            balance.mature_utxo_count,
            balance.pending_utxo_count,
            pending_transition_balance_formatted,
        ),
    )
    .await?;

    return Ok(());
}
