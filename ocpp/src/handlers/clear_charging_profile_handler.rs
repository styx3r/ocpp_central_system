use std::sync::{Arc, Mutex};
use crate::ChargePointState;

use rust_ocpp::v1_6::messages::clear_charging_profile;
use rust_ocpp::v1_6::types::ClearChargingProfileStatus;

use log::{info, warn};

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_clear_charging_profile_response(
    response_uuid: &String,
    clear_charging_profile_response: &clear_charging_profile::ClearChargingProfileResponse,
    charge_point_state: &mut ChargePointState,
) {
    match clear_charging_profile_response.status {
        ClearChargingProfileStatus::Accepted => {
            info!(
                "ClearChargingProfileStatus request with UUID {} has been accepted by the ChargingPoint",
                response_uuid
            );
        }
        ClearChargingProfileStatus::Unknown => {
            warn!(
                "ClearChargingProfileStatus request with UUID {} has been rejected by the ChargingPoint because ChargingProfile is not known",
                response_uuid
            );
        }
    }

    charge_point_state
        .requests_awaiting_confirmation
        .retain(|e| *e.uuid != *response_uuid);
}
