use crate::ChargePointState;

use rust_ocpp::v1_6::messages::change_configuration;
use rust_ocpp::v1_6::types::ConfigurationStatus;

use log::{error, info, warn};

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_change_configuration_response(
    response_uuid: &String,
    change_configuration_response: &change_configuration::ChangeConfigurationResponse,
    charge_point_state: &mut ChargePointState,
) {
    match change_configuration_response.status {
        ConfigurationStatus::Accepted => {
            info!(
                "ChangeConfiguration request with UUID {} has been accepted by the ChargingPoint",
                response_uuid
            );
        }
        ConfigurationStatus::NotSupported => {
            warn!(
                "ChangeConfiguration request with UUID {} is not supported by the ChargingPoint",
                response_uuid
            );
        }
        ConfigurationStatus::RebootRequired => {
            warn!(
                "ChangeConfiguration request with UUID {} requires a reboot of the ChargingPoint",
                response_uuid
            );
        }
        ConfigurationStatus::Rejected => {
            error!(
                "ChangeConfiguration request with UUID {} has been rejected by the ChargingPoint",
                response_uuid
            );
        }
    }

    charge_point_state
        .requests_awaiting_confirmation
        .retain(|e| *e.uuid != *response_uuid);
}
