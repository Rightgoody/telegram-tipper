use std::sync::Arc;

use crate::error::Error;
use crate::tip_context::TipContext;
use crate::utils::connect_wallet_to_rpc;
use crate::{owned_wallet_metadata::OwnedWalletMetadata, result::Result};
use spectre_addresses::Address;
use spectre_wallet_core::{
    api::WalletApi,
    deterministic::bip32::BIP32_ACCOUNT_KIND,
    prelude::{EncryptionKind, Language, Mnemonic, WordCount},
    storage::PrvKeyData,
    wallet::{AccountCreateArgsBip32, Wallet, WalletCreateArgs, WalletOpenArgs},
};
use spectre_wallet_keys::secret::Secret;

#[derive(Clone)]
pub struct TipOwnedWallet {
    owned_identifier: String,
    wallet: Arc<Wallet>,
    receive_address: Address,
}

impl TipOwnedWallet {
    pub fn new(owned_identifier: String, wallet: Arc<Wallet>, receive_address: Address) -> Self {
        TipOwnedWallet {
            owned_identifier,
            receive_address,
            wallet,
        }
    }

    pub async fn create(
        tip_context: Arc<TipContext>,
        wallet_secret: &Secret,
        owned_identifier: &str,
    ) -> Result<(TipOwnedWallet, Mnemonic)> {
        let mnemonic = Mnemonic::random(WordCount::Words12, Language::default())?;
        let localstore = Wallet::local_store()?;

        let wallet = Wallet::try_new(
            localstore,
            Some(tip_context.resolver()),
            Some(tip_context.network_id()),
        )?;

        let wallet_arc = Arc::new(wallet.clone());

        let wallet_args: WalletCreateArgs = WalletCreateArgs::new(
            Some(owned_identifier.into()),
            None,
            EncryptionKind::XChaCha20Poly1305,
            None,
            true,
        );

        wallet_arc.store().batch().await?;

        wallet_arc.create_wallet(wallet_secret, wallet_args).await?;

        let prv_key_data = PrvKeyData::try_from_mnemonic(
            mnemonic.clone(),
            None,
            // unused since payment_secret is None
            EncryptionKind::XChaCha20Poly1305,
        )?;
        let prv_key_data_id = prv_key_data.id;

        let prv_key_data_store = wallet_arc.store().as_prv_key_data_store()?;
        prv_key_data_store
            .store(wallet_secret, prv_key_data)
            .await?;
        wallet_arc.store().commit(wallet_secret).await?;

        let account_args = AccountCreateArgsBip32::new(None, None);
        let account = wallet_arc
            .create_account_bip32(wallet_secret, prv_key_data_id, None, account_args)
            .await?;

        let receive_address = account.receive_address()?;

        wallet_arc.store().flush(wallet_secret).await?;

        {
            let guard = wallet_arc.guard();
            let guard = guard.lock().await;
            wallet_arc.activate_accounts(None, &guard).await?;

            connect_wallet_to_rpc(&wallet_arc, tip_context.rpc_api()).await?;

            wallet_arc.start().await?;
        }

        wallet_arc.autoselect_default_account_if_single().await?;

        wallet_arc
            .account()?
            .utxo_context()
            .register_addresses(&[receive_address.clone()])
            .await?;

        let tip_wallet = TipOwnedWallet::new(owned_identifier.into(), wallet_arc, receive_address);

        tip_context
            .owned_wallet_metadata_store
            .add(&OwnedWalletMetadata::new(
                owned_identifier.into(),
                tip_wallet.receive_address(),
            ))
            .await?;

        let tip_owned_wallet =
            tip_context.add_opened_owned_wallet(owned_identifier.into(), tip_wallet);

        Ok((tip_owned_wallet, mnemonic))
    }

    pub async fn open(
        tip_context: Arc<TipContext>,
        wallet_secret: &Secret,
        owned_identifier: &str,
    ) -> Result<TipOwnedWallet> {
        let localstore = Wallet::local_store()?;

        let wallet = Wallet::try_new(
            localstore,
            Some(tip_context.resolver()),
            Some(tip_context.network_id()),
        )?;
        let wallet_arc = Arc::new(wallet.clone());

        let args = WalletOpenArgs::default_with_legacy_accounts();

        {
            let guard = wallet_arc.guard();
            let guard = guard.lock().await;

            connect_wallet_to_rpc(&wallet_arc, tip_context.rpc_api()).await?;

            wallet_arc
                .open(wallet_secret, Some(owned_identifier.into()), args, &guard)
                .await?;

            wallet_arc.start().await?;

            wallet_arc.activate_accounts(None, &guard).await?;
            wallet_arc.autoselect_default_account_if_single().await?;
        }

        let receive_address = wallet_arc.account()?.receive_address()?;

        wallet_arc
            .account()?
            .utxo_context()
            .register_addresses(&[receive_address.clone()])
            .await?;

        let tip_wallet = TipOwnedWallet::new(owned_identifier.into(), wallet_arc, receive_address);

        let tip_owned_wallet =
            tip_context.add_opened_owned_wallet(owned_identifier.into(), tip_wallet);

        Ok(tip_owned_wallet)
    }

    /**
     * restore a wallet from a mnemonic
     * override any already existing wallet owned by `owned_identifier`
     */
    pub async fn restore(
        tip_context: Arc<TipContext>,
        wallet_secret: &Secret,
        mnemonic: Mnemonic,
        owned_identifier: &str,
    ) -> Result<TipOwnedWallet> {
        let localstore = Wallet::local_store()?;

        let wallet = Wallet::try_new(
            localstore,
            Some(tip_context.resolver()),
            Some(tip_context.network_id()),
        )?;

        let wallet_arc = Arc::new(wallet.clone());

        let wallet_args: WalletCreateArgs = WalletCreateArgs::new(
            Some(owned_identifier.into()),
            None,
            EncryptionKind::XChaCha20Poly1305,
            None,
            true,
        );

        wallet_arc.store().batch().await?;

        wallet_arc.create_wallet(wallet_secret, wallet_args).await?;

        let prv_key_data = PrvKeyData::try_from_mnemonic(
            mnemonic.clone(),
            None,
            // unused since payment_secret is None
            EncryptionKind::XChaCha20Poly1305,
        )?;
        let prv_key_data_id = prv_key_data.id;

        let prv_key_data_store = wallet_arc.store().as_prv_key_data_store()?;
        prv_key_data_store
            .store(wallet_secret, prv_key_data)
            .await?;
        wallet_arc.store().commit(wallet_secret).await?;

        let account_args = AccountCreateArgsBip32::new(None, None);
        let account = wallet_arc
            .create_account_bip32(wallet_secret, prv_key_data_id, None, account_args)
            .await?;

        let receive_address = account.receive_address()?;

        wallet_arc.store().flush(wallet_secret).await?;

        {
            let guard = wallet_arc.guard();
            let guard = guard.lock().await;
            wallet_arc.activate_accounts(None, &guard).await?;

            connect_wallet_to_rpc(&wallet_arc, tip_context.rpc_api()).await?;

            wallet_arc.start().await?;
        }

        wallet_arc.autoselect_default_account_if_single().await?;

        wallet_arc
            .account()?
            .utxo_context()
            .register_addresses(&[receive_address.clone()])
            .await?;

        let tip_owned_wallet =
            TipOwnedWallet::new(owned_identifier.into(), wallet_arc, receive_address);

        tip_context
            .owned_wallet_metadata_store
            .remove_by_owner_identifier(owned_identifier.into())
            .await?;

        tip_context
            .owned_wallet_metadata_store
            .add(&OwnedWalletMetadata::new(
                owned_identifier.into(),
                tip_owned_wallet.receive_address(),
            ))
            .await?;

        let tip_owned_wallet =
            tip_context.add_opened_owned_wallet(owned_identifier.into(), tip_owned_wallet);

        Ok(tip_owned_wallet)
    }

    pub fn owned_identifier(&self) -> &str {
        &self.owned_identifier
    }

    pub fn wallet(&self) -> Arc<Wallet> {
        self.wallet.clone()
    }

    pub fn receive_address(&self) -> Address {
        self.receive_address.clone()
    }

    /// change secret
    pub async fn change_secret(&self, old_secret: &Secret, new_secret: &Secret) -> Result<()> {
        self.wallet
            .clone()
            .wallet_change_secret(old_secret.clone(), new_secret.clone())
            .await?;
        Ok(())
    }

    /// export mnemonic with xpub
    pub async fn export_mnemonic_and_xpub(
        &self,
        wallet_secret: &Secret,
    ) -> Result<(Option<Mnemonic>, String)> {
        let account = self.wallet.account()?;
        let prv_key_data_id = account.prv_key_data_id()?;

        let prv_key_data_store = self.wallet.store().as_prv_key_data_store()?;
        let prv_key_data = prv_key_data_store
            .load_key_data(wallet_secret, prv_key_data_id)
            .await?
            .ok_or(Error::OwnedWalletNotFound())?;

        let mnemonic = prv_key_data
            .as_mnemonic(None)
            .map_err(|_| Error::OwnedWalletNotFound())?;

        let xpub_key = prv_key_data
            .create_xpub(None, BIP32_ACCOUNT_KIND.into(), 0)
            .await?;

        let xpub_formatted = self.wallet.network_format_xpub(&xpub_key);

        Ok((mnemonic, xpub_formatted))
    }
}

#[cfg(test)]
mod tests {
    use spectre_wrpc_client::{
        prelude::{NetworkId, NetworkType},
        Resolver,
    };

    use super::*;

    fn get_ctx() -> Arc<TipContext> {
        TipContext::new_arc(Resolver::default(), NetworkId::new(NetworkType::Mainnet))
    }

    #[tokio::test]
    async fn test_create_wallet() {
        TipOwnedWallet::create(get_ctx(), &Secret::from("value"), "identifier")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_open_wallet() {
        TipOwnedWallet::create(get_ctx(), &Secret::from("value"), "identifier2")
            .await
            .unwrap();
        TipOwnedWallet::open(get_ctx(), &Secret::from("value"), "identifier2")
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_open_wallet_with_wrong_secret() {
        let _ = TipOwnedWallet::create(get_ctx(), &Secret::from("value"), "identifier3").await;
        TipOwnedWallet::open(get_ctx(), &Secret::from("value2"), "identifier3")
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_open_wallet_with_innexistant_wallet() {
        TipOwnedWallet::open(get_ctx(), &Secret::from("value2"), "identifier_innexistant")
            .await
            .unwrap();
    }
}
