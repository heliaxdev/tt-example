use namada_sdk::{
    address::Address, args::{
        InputAmount, TxBuilder, TxExpiration, TxShieldingTransferData, TxUnshieldingTransferData,
    }, key::common, masp_primitives::transaction::components::sapling::builder::RngBuildParams, signing::default_sign, time::DateTimeUtc, token::{self, DenominatedAmount}, tx::data::GasLimit, ExtendedSpendingKey, Namada, PaymentAddress, DEFAULT_GAS_LIMIT
};
use rand_core::OsRng;

use crate::{sdk::Sdk, utils};

pub async fn execute_unshielding_tx(
    sdk: &Sdk,
    source_address: Address,
    spending_key: ExtendedSpendingKey,
    token_address: Address,
    gas_payer: common::PublicKey,
    signers: Vec<common::PublicKey>,
    amount: token::Amount,
    memo: Option<String>,
    expiration: Option<i64>,
) -> Result<bool, String> {
    let tx_transfer_data = TxUnshieldingTransferData {
        target: source_address,
        token: token_address.clone(),
        amount: InputAmount::Validated(DenominatedAmount::native(amount)),
    };

    let bparams = RngBuildParams::new(OsRng);

    let mut transfer_tx_builder =
        sdk.namada
            .new_unshielding_transfer(spending_key, vec![tx_transfer_data], None, false);
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

    let (mut transfer_tx, signing_data) = transfer_tx_builder
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
