use rust_ocpp::v1_6::messages::remote_start_transaction;
use rust_ocpp::v1_6::types::RemoteStartStopStatus;

use log::{info, warn};

use crate::ocpp::ChargePointState;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_remote_start_transaction_response(
    response_uuid: &String,
    remote_start_transaction_response: &remote_start_transaction::RemoteStartTransactionResponse,
    charge_point_state: &mut ChargePointState,
) {
    match remote_start_transaction_response.status {
        RemoteStartStopStatus::Accepted => {
            info!(
                "RemoteStartStopStatus request with UUID {} has been accepted by the ChargingPoint",
                response_uuid
            );
        }
        RemoteStartStopStatus::Rejected => {
            warn!(
                "RemoteStartStopStatus request with UUID {} has been rejected by the ChargingPoint",
                response_uuid
            );
        }
    }

    charge_point_state
        .requests_awaiting_confirmation
        .retain(|e| *e.uuid != *response_uuid);
}
