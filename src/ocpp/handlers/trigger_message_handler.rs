use rust_ocpp::v1_6::messages::trigger_message;
use rust_ocpp::v1_6::types::TriggerMessageStatus;

use log::{info, warn, error};

use crate::ocpp::ChargePointState;

//------------------------------------------------------------------------------------------------

pub(crate) fn handle_trigger_message_response(
    response_uuid: &String,
    trigger_message_response: &trigger_message::TriggerMessageResponse,
    charge_point_state: &mut ChargePointState
) {
    match trigger_message_response.status {
        TriggerMessageStatus::Accepted => {
            info!(
                "OCCP request with UUID {} has been accepted by the ChargingPoint",
                response_uuid
            );
        }
        TriggerMessageStatus::Rejected => {
            warn!(
                "OCCP request with UUID {} has been rejected by the ChargingPoint",
                response_uuid
            );
        }
        TriggerMessageStatus::NotImplemented => {
            error!(
                "OCPP request with UUID {} is not implemented on ChargingPoint",
                response_uuid
            );
        }
    }

    charge_point_state.requests_awaiting_confirmation
        .retain(|e| *e.uuid != *response_uuid);
}
