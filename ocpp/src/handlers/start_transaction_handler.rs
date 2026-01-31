use rust_ocpp::v1_6::messages::start_transaction;
use rust_ocpp::v1_6::types::{AuthorizationStatus, IdTagInfo};

use crate::ocpp_types::CustomError;
use crate::{ChargePointState, Transaction};
use config::config::IdTag;

//------------------------------------------------------------------------------------------------

pub(crate) fn handle_start_transaction_request(
    start_transaction: &start_transaction::StartTransactionRequest,
    authorized_id_tags: &Vec<IdTag>,
    charge_point_state: &mut ChargePointState,
) -> Result<start_transaction::StartTransactionResponse, CustomError> {
    let authorization_status = match authorized_id_tags
        .iter()
        .find(|e| e.id == start_transaction.id_tag)
    {
        Some(_) => AuthorizationStatus::Accepted,
        None => AuthorizationStatus::Invalid,
    };

    let transaction_id = (charge_point_state.running_transactions.len() + 1) as i32;
    let start_transaction_response = start_transaction::StartTransactionResponse {
        id_tag_info: IdTagInfo {
            expiry_date: None,
            parent_id_tag: None,
            status: authorization_status.clone(),
        },
        transaction_id,
    };

    if authorization_status == AuthorizationStatus::Accepted {
        charge_point_state.running_transactions.push(Transaction {
            id_tag: Some(start_transaction.id_tag.clone()),
            transaction_id,
        });
    }

    Ok(start_transaction_response)
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    static UNITTEST_CONNECTOR_ID: u32 = 1;
    static UNITTEST_ID_TAG: &str = "ID_TAG";

    #[test]
    fn start_transaction_request() -> Result<(), CustomError> {
        let mut charge_point_state = ChargePointState::default();
        let response = handle_start_transaction_request(
            &start_transaction::StartTransactionRequest {
                connector_id: UNITTEST_CONNECTOR_ID,
                id_tag: UNITTEST_ID_TAG.to_owned(),
                meter_start: 0,
                reservation_id: None,
                timestamp: chrono::offset::Utc::now(),
            },
            &vec![],
            &mut charge_point_state,
        )?;

        assert_eq!(
            response.id_tag_info,
            IdTagInfo {
                expiry_date: None,
                parent_id_tag: None,
                status: AuthorizationStatus::Blocked
            }
        );

        Ok(())
    }
}
