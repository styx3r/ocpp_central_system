use rust_ocpp::v1_6::{
    messages::stop_transaction,
    types::{AuthorizationStatus, IdTagInfo},
};

use crate::ocpp_types::CustomError;
use crate::ChargePointState;

//------------------------------------------------------------------------------------------------

pub(crate) fn handle_stop_transaction_request(
    stop_transaction_request: &stop_transaction::StopTransactionRequest,
    charge_point_state: &mut ChargePointState,
) -> Result<stop_transaction::StopTransactionResponse, CustomError> {
    let (authorization_status, transaction) = match charge_point_state
        .running_transactions
        .iter_mut()
        .find(|e| e.transaction_id == stop_transaction_request.transaction_id)
    {
        Some(transaction) => {
            transaction.meter_value_stop = stop_transaction_request.meter_stop;
            (AuthorizationStatus::Accepted, Some(transaction.clone()))
        }
        _ => (AuthorizationStatus::Invalid, None),
    };

    if authorization_status == AuthorizationStatus::Accepted && let Some(transaction) = transaction {
        charge_point_state
            .running_transactions
            .retain(|e| *e != transaction);
    }

    Ok(stop_transaction::StopTransactionResponse {
        id_tag_info: Some(IdTagInfo {
            expiry_date: None,
            parent_id_tag: None,
            status: authorization_status,
        }),
    })
}

//------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_ocpp::v1_6::types::Reason;
    use crate::charge_point_state::Transaction;

    static UNITTEST_ID_TAG: &str = "ID_TAG";
    static UNITTEST_TRANSACTION_ID: i32 = 1;

    #[test]
    fn stop_transaction_request_without_running_transaction() -> Result<(), CustomError> {
        let mut charge_point_state = ChargePointState::default();
        let response = handle_stop_transaction_request(
            &stop_transaction::StopTransactionRequest {
                id_tag: Some(UNITTEST_ID_TAG.to_owned()),
                meter_stop: 1000,
                timestamp: chrono::offset::Utc::now(),
                transaction_id: 1,
                reason: Some(Reason::Local),
                transaction_data: None,
            },
            &mut charge_point_state,
        )?;

        assert_eq!(
            response.id_tag_info,
            Some(IdTagInfo {
                expiry_date: None,
                parent_id_tag: None,
                status: AuthorizationStatus::Invalid
            })
        );

        Ok(())
    }

    #[test]
    fn stop_transaction_request_with_running_transaction_and_id_tag() -> Result<(), CustomError> {
        let mut charge_point_state = ChargePointState::default();
        charge_point_state.running_transactions.push(Transaction {
            id_tag: Some(UNITTEST_ID_TAG.to_string()),
            transaction_id: UNITTEST_TRANSACTION_ID,
            meter_value_start: 0,
            meter_value_stop: 0
        });

        let response = handle_stop_transaction_request(
            &stop_transaction::StopTransactionRequest {
                id_tag: Some(UNITTEST_ID_TAG.to_string()),
                meter_stop: 1000,
                timestamp: chrono::offset::Utc::now(),
                transaction_id: UNITTEST_TRANSACTION_ID,
                reason: Some(Reason::Local),
                transaction_data: None,
            },
            &mut charge_point_state,
        )?;

        assert_eq!(
            response.id_tag_info,
            Some(IdTagInfo {
                expiry_date: None,
                parent_id_tag: None,
                status: AuthorizationStatus::Accepted
            })
        );

        Ok(())
    }

    #[test]
    fn stop_transaction_request_with_running_transaction_and_no_id_tag() -> Result<(), CustomError>
    {
        let mut charge_point_state = ChargePointState::default();
        charge_point_state.running_transactions.push(Transaction {
            id_tag: Some(UNITTEST_ID_TAG.to_string()),
            transaction_id: UNITTEST_TRANSACTION_ID,
            meter_value_start: 0,
            meter_value_stop: 0
        });

        let response = handle_stop_transaction_request(
            &stop_transaction::StopTransactionRequest {
                id_tag: None,
                meter_stop: 1000,
                timestamp: chrono::offset::Utc::now(),
                transaction_id: UNITTEST_TRANSACTION_ID,
                reason: Some(Reason::Local),
                transaction_data: None,
            },
            &mut charge_point_state,
        )?;

        assert_eq!(
            response.id_tag_info,
            Some(IdTagInfo {
                expiry_date: None,
                parent_id_tag: None,
                status: AuthorizationStatus::Accepted
            })
        );

        Ok(())
    }
}
