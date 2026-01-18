use rust_ocpp::v1_6::messages::set_charging_profile;
use rust_ocpp::v1_6::types::ChargingProfileStatus;

use log::{error, info, warn};

use crate::ocpp::ChargePointState;

//------------------------------------------------------------------------------------------------

pub(crate) fn handle_set_charging_profile_response(
    response_uuid: &String,
    set_charging_profile_response: &set_charging_profile::SetChargingProfileResponse,
    charge_point_state: &mut ChargePointState,
) {
    match set_charging_profile_response.status {
        ChargingProfileStatus::Accepted => {
            info!(
                "SetChargingProfile request with UUID {} has been accepted by the ChargingPoint",
                response_uuid
            );
        }
        ChargingProfileStatus::Rejected => {
            warn!(
                "SetChargingProfile request with UUID {} has been rejected by the ChargingPoint",
                response_uuid
            );
        }
        ChargingProfileStatus::NotSupported => {
            error!(
                "SetChargingProfile request with UUID {} is not supported by the ChargingPoint",
                response_uuid
            );
        }
    }

    charge_point_state
        .requests_awaiting_confirmation
        .retain(|e| *e.uuid != *response_uuid);
}
