use namada_sdk::{
    address::Address, args::{InputAmount, TxBuilder, TxExpiration, TxTransparentTransferData}, borsh::BorshDeserialize, bytes::HEXLOWER, key::common, signing::default_sign, time::DateTimeUtc, token::{self, DenominatedAmount, Transfer}, tx::data::GasLimit, Namada, DEFAULT_GAS_LIMIT
};

use crate::{sdk::Sdk, utils};

pub async fn execute_transparent_tx(
    sdk: &Sdk,
    source_address: Address,
    target_address: Address,
    token_address: Address,
    gas_payer: common::PublicKey,
    signers: Vec<common::PublicKey>,
    amount: token::Amount,
    memo: Option<String>,
    expiration: Option<i64>,
) -> Result<bool, String> {
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

    let (mut transfer_tx, signing_data) = transfer_tx_builder
        .build(&sdk.namada)
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

<<<<<<< Updated upstream
    tracing::debug!("tx result: {:?}", tx);

=======
<<<<<<< Updated upstream
=======
    tracing::info!("Transparent wrapper tx hash: {:?}", transfer_tx.wrapper_hash().map(|h| HEXLOWER.encode(&h.0)));

    tracing::debug!("tx result: {:?}", tx);

>>>>>>> Stashed changes
>>>>>>> Stashed changes
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
