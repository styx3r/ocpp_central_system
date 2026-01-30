use rust_ocpp::v1_6::messages::remote_stop_transaction;
use rust_ocpp::v1_6::types::RemoteStartStopStatus;

use log::{info, warn};

use crate::ChargePointState;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_remote_stop_transaction_response(
    response_uuid: &String,
    remote_stop_transaction_response: &remote_stop_transaction::RemoteStopTransactionResponse,
    charge_point_state: &mut ChargePointState,
) {
    match remote_stop_transaction_response.status {
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
