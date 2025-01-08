use namada_sdk::{args::TxBuilder, key::common, signing::default_sign, tx::data::GasLimit, Namada, DEFAULT_GAS_LIMIT};

use crate::{sdk::Sdk, utils};

pub async fn execute_reveal_pk(
    sdk: &Sdk,
    public_key: common::PublicKey,
) -> Result<bool, String> {
    let reveal_pk_tx_builder = sdk
        .namada
        .new_reveal_pk(public_key.clone())
        .signing_keys(vec![public_key.clone()])
        .gas_limit(GasLimit::from(DEFAULT_GAS_LIMIT))
        .wrapper_fee_payer(public_key);

    let (mut reveal_tx, signing_data) = reveal_pk_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| e.to_string())?;

    sdk.namada
        .sign(
            &mut reveal_tx,
            &reveal_pk_tx_builder.tx,
            signing_data,
            default_sign,
            (),
        )
        .await
        .expect("unable to sign tx");

    let tx = sdk
        .namada
        .submit(reveal_tx.clone(), &reveal_pk_tx_builder.tx)
        .await;

    if utils::is_tx_rejected(&reveal_tx, &tx) {
        match tx {
            Ok(tx) => {
                let errors = utils::get_tx_errors(&reveal_tx, &tx).unwrap_or_default();
                return Err(errors);
            }
            Err(e) => return Err(e.to_string()),
        }
    } else {
        Ok(true)
    }
}