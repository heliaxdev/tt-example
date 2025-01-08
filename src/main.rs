use std::{env, str::FromStr, time::Duration};

use clap::Parser;
use config::AppConfig;
use namada_sdk::{
    address::Address,
    io::NullIo,
    key::common::SecretKey,
    masp::{fs::FsShieldedUtils, ShieldedContext},
    rpc,
    token::{self, Amount},
    wallet::fs::FsWalletUtils,
};
use reveal_pk::execute_reveal_pk;
use sdk::Sdk;
use tendermint_rpc::{HttpClient, Url};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use transparent_transfer::execute_transparent_tx;

pub mod config;
pub mod reveal_pk;
pub mod sdk;
pub mod transparent_transfer;
pub mod utils;

#[tokio::main]
async fn main() {
    let config = AppConfig::parse();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .unwrap();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .without_time()
        .with_ansi(false)
        .init();

    let url = Url::from_str(&config.rpc).expect("invalid RPC address");
    let http_client = HttpClient::new(url).unwrap();

    // we initialize a Sdk structure
    let sdk = loop {
        let base_dir = config
            .base_dir
            .clone()
            .unwrap_or(env::current_dir().unwrap());
        // Setup wallet storage
        let mut wallet = FsWalletUtils::new(base_dir.clone());
        if base_dir.join("wallet.toml").exists() {
            wallet.load().expect("Should be able to load the wallet;");
        }

        let mut shielded_ctx = ShieldedContext::new(FsShieldedUtils::new(base_dir.clone()));
        if base_dir.join("shielded.dat").exists() {
            shielded_ctx
                .load()
                .await
                .expect("Should be able to load shielded context");
        } else {
            shielded_ctx.save().await.unwrap();
        }

        let io = NullIo;

        match Sdk::new(&config, http_client.clone(), wallet, shielded_ctx, io).await {
            Ok(sdk) => break sdk,
            Err(_) => std::thread::sleep(Duration::from_secs(2)),
        };
    };

    let native_token = rpc::query_native_token(&sdk.namada.clone_client())
        .await
        .unwrap();

    // we now have an sdk, lets derive an address from the source private key and reveal the correspoding public key (if is not already revealed)

    let source_private_key = SecretKey::from_str(&config.source_private_key).unwrap();
    let source_public_key = source_private_key.to_public();
    let source_address = Address::from(&source_public_key);

    tracing::info!("Check {} nam balance...", source_address);
    let balance = rpc::get_token_balance(
        &sdk.namada.clone_client(),
        &native_token,
        &source_address,
        None,
    )
    .await
    .unwrap_or_default();

    if balance.is_zero() || !balance.can_spend(&Amount::from_u64(config.amount)) {
        tracing::error!(
            "Not enough balance (got {}unam, neeeded {}unam)",
            balance,
            config.amount
        );
        std::process::exit(1);
    } else {
        tracing::info!("Balance is {}unam", balance);
    }

    // we could also fetch the public key via wallet
    // let wallet = sdk.namada.wallet.read().await;
    // let source_public_key = wallet.find_public_key("source").unwrap();
    // drop(wallet);

    tracing::info!(
        "Checkin if {} needs to reveal the public key...",
        source_public_key
    );

    // check if public key is already revealed
    let is_public_key_already_revealed =
        rpc::is_public_key_revealed(&sdk.namada.clone_client(), &source_address)
            .await
            .unwrap_or(false);

    if !is_public_key_already_revealed {
        tracing::info!("Revealing public key...");
        execute_reveal_pk(&sdk, source_public_key.clone())
            .await
            .unwrap();
        tracing::info!("Public key revealed!");
    } else {
        tracing::info!("Public key already revealed!");
    }

    tracing::info!("Building transfer transaction...");

    // we can now make the transfer
    let target_address = Address::from_str(&config.target_address).unwrap();

    let fee_payer = source_public_key.clone();
    let token_amount = token::Amount::from_u64(config.amount);

    tracing::info!("Executing transfer transaction...");

    execute_transparent_tx(
        &sdk,
        source_address,
        target_address,
        native_token,
        fee_payer,
        vec![source_public_key],
        token_amount,
        config.memo,
        config.expiration_timestamp_utc,
    )
    .await
    .unwrap();

    tracing::info!("Done!");
}
