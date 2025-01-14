use namada_sdk::{
    address::Address,
    args::{InputAmount, TxBuilder, TxExpiration, TxShieldingTransferData},
    bytes::HEXLOWER,
    key::common,
    masp_primitives::transaction::components::sapling::builder::{BuildParams, RngBuildParams},
    signing::default_sign,
    time::DateTimeUtc,
    token::{self, DenominatedAmount},
    tx::data::GasLimit,
    Namada, PaymentAddress, DEFAULT_GAS_LIMIT,
};
use rand_core::OsRng;

use crate::{sdk::Sdk, utils};

pub async fn execute_shielding_tx(
    sdk: &Sdk,
    source_address: Address,
    target_address: PaymentAddress,
    token_address: Address,
    gas_payer: common::PublicKey,
    signers: Vec<common::PublicKey>,
    amount: token::Amount,
    memo: Option<String>,
    expiration: Option<i64>,
) -> Result<bool, String> {
    let tx_transfer_data = TxShieldingTransferData {
        source: source_address.clone(),
        token: token_address,
        amount: InputAmount::Unvalidated(DenominatedAmount::native(amount)),
    };

    let mut bparams = RngBuildParams::new(OsRng);

    let mut transfer_tx_builder = sdk
        .namada
        .new_shielding_transfer(target_address, vec![tx_transfer_data]);
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

    let (mut transfer_tx, signing_data, _epoch) = transfer_tx_builder
        .build(&sdk.namada, &mut bparams)
        .await
        .map_err(|e| e.to_string())?;

    sdk.namada
        .sign(
            &mut transfer_tx,
            &transfer_tx_builder.tx,
            signing_data,
            default_sign,
            (),
        )
        .await
        .expect("unable to sign tx");

    let tx = sdk
        .namada
        .submit(transfer_tx.clone(), &transfer_tx_builder.tx)
        .await;

    tracing::info!(
        "Shielding wrapper tx hash: {:?}",
        transfer_tx.wrapper_hash().map(|h| HEXLOWER.encode(&h.0))
    );

    tracing::debug!("tx result: {:?}", tx);

    if utils::is_tx_rejected(&transfer_tx, &tx) {
        match tx {
            Ok(tx) => {
                let errors = utils::get_tx_errors(&transfer_tx, &tx).unwrap_or_default();
                return Err(errors);
            }
            Err(e) => return Err(e.to_string()),
        }
    } else {
        Ok(true)
    }
}
