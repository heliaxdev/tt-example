use namada_sdk::{
    address::Address,
    args::{InputAmount, TxBuilder, TxExpiration, TxTransparentTransferData},
    bytes::HEXLOWER,
    key::common,
    signing::{find_key_by_pk, OfflineSignatures, SigningTxData},
    time::DateTimeUtc,
    token::{self, DenominatedAmount},
    tx::data::GasLimit,
    Namada, DEFAULT_GAS_LIMIT,
};

use crate::{sdk::Sdk, utils};

pub async fn build_transparent_transfer(
    sdk: &Sdk,
    source_address: Address,
    target_address: Address,
    token_address: Address,
    gas_payer: common::PublicKey,
    signers: Vec<common::PublicKey>,
    amount: token::Amount,
    memo: Option<String>,
    expiration: Option<i64>,
) -> Result<(namada_sdk::args::Tx, namada_sdk::tx::Tx, SigningTxData), String> {
    let tx_transfer_data = TxTransparentTransferData {
        source: source_address.clone(),
        target: target_address.clone(),
        token: token_address,
        amount: InputAmount::Unvalidated(DenominatedAmount::native(amount)),
    };

    let mut transfer_tx_builder = sdk.namada.new_transparent_transfer(vec![tx_transfer_data]);
    transfer_tx_builder = transfer_tx_builder.gas_limit(GasLimit::from(DEFAULT_GAS_LIMIT));
    transfer_tx_builder = transfer_tx_builder.wrapper_fee_payer(gas_payer);
    if let Some(memo) = memo {
        transfer_tx_builder = transfer_tx_builder.memo(memo.as_bytes().to_vec())
    }
    if let Some(expiration) = expiration {
        transfer_tx_builder = transfer_tx_builder.expiration(TxExpiration::Custom(
            DateTimeUtc::from_unix_timestamp(expiration).unwrap(),
        ));
    }
    transfer_tx_builder = transfer_tx_builder.signing_keys(signers);

    let (tx, signing_tx_data) = transfer_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| e.to_string())?;

    Ok((transfer_tx_builder.tx, tx, signing_tx_data))
}

pub async fn generate_offline_signatures(
    sdk: &Sdk,
    args: &namada_sdk::args::Tx,
    tx: namada_sdk::tx::Tx,
    signing_tx_data: SigningTxData,
) -> Result<OfflineSignatures, String> {
    let mut wallet = sdk.namada.wallet_mut().await;
    let secret_keys_res: Vec<_> = signing_tx_data
        .public_keys
        .iter()
        .map(|pubkey| find_key_by_pk(&mut wallet, args, pubkey))
        .collect();
    let secret_keys: Result<Vec<common::SecretKey>, _> = secret_keys_res
        .into_iter()
        .map(|res| res.map_err(|e| e.to_string()))
        .collect();
    let wrapper_signer = find_key_by_pk(&mut *wallet, args, &signing_tx_data.fee_payer).ok();

    namada_sdk::signing::generate_tx_signatures(
        tx,
        secret_keys?,
        signing_tx_data.owner,
        wrapper_signer,
    )
    .await
    .map_err(|err| err.to_string())
}

pub async fn submit_transparent_tx(
    sdk: &Sdk,
    args: &namada_sdk::args::Tx,
    mut transfer_tx: namada_sdk::tx::Tx,
    OfflineSignatures {
        signatures,
        wrapper_signature,
    }: OfflineSignatures,
) -> Result<bool, String> {
    // Attach the signatures produced offline
    transfer_tx.add_signatures(signatures);
    if let Some(auth) = wrapper_signature {
        transfer_tx.add_section(namada_sdk::tx::Section::Authorization(auth));
    }

    // Submit tx
    let tx = sdk.namada.submit(transfer_tx.clone(), args).await;

    tracing::info!(
        "Transparent wrapper tx hash: {:?}",
        transfer_tx.wrapper_hash().map(|h| HEXLOWER.encode(&h.0))
    );

    tracing::debug!("tx result: {:?}", tx);

    if utils::is_tx_rejected(&transfer_tx, &tx) {
        match tx {
            Ok(tx) => {
                let errors = utils::get_tx_errors(&transfer_tx, &tx).unwrap_or_default();
                Err(errors)
            }
            Err(e) => Err(e.to_string()),
        }
    } else {
        Ok(true)
    }
}
