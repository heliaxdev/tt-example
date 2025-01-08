use std::str::FromStr;

use namada_sdk::io::NamadaIo;
use namada_sdk::wallet::fs::FsWalletUtils;
use namada_sdk::wallet::Wallet;
use namada_sdk::{
    address::{Address, ImplicitAddress},
    args::TxBuilder,
    chain::ChainId,
    io::NullIo,
    key::common::SecretKey,
    masp::{fs::FsShieldedUtils, ShieldedContext},
    rpc, NamadaImpl,
};
use tendermint_rpc::HttpClient;

use crate::config::AppConfig;

// thi structure is a wrapper around a Namada Sdk
pub struct Sdk {
    pub namada: NamadaImpl<HttpClient, FsWalletUtils, FsShieldedUtils, NullIo>,
}

impl Sdk {
    // creating an Sdk with and storing in the wallet the source address private key with alias `source` and the native token as `nam`
    pub async fn new(
        config: &AppConfig,
        http_client: HttpClient,
        wallet: Wallet<FsWalletUtils>,
        shielded_ctx: ShieldedContext<FsShieldedUtils>,
        io: NullIo,
    ) -> Result<Sdk, String> {
        let sk = SecretKey::from_str(&config.source_private_key).unwrap();
        let public_key = sk.to_public();
        let address = Address::Implicit(ImplicitAddress::from(&public_key));

        let namada = NamadaImpl::new(http_client, wallet, shielded_ctx.into(), io)
            .await
            .map_err(|e| e.to_string())?;
        let namada = namada.chain_id(ChainId::from_str(&config.chain_id).unwrap());

        let mut namada_wallet = namada.wallet.write().await;
        namada_wallet
            .insert_keypair("source".to_string(), true, sk, None, Some(address), None)
            .unwrap();

        let native_token = rpc::query_native_token(namada.client())
            .await
            .map_err(|e| e.to_string())?;
        namada_wallet
            .insert_address("nam", native_token, true)
            .unwrap();
        drop(namada_wallet);

        Ok(Self { namada })
    }
}
