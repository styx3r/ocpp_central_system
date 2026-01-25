use rust_ocpp::v1_6::messages::get_diagnostics;

use log::{info, warn};

use crate::ChargePointState;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_get_diagnostics_response(
    response_uuid: &String,
    get_diagnostics_response: &get_diagnostics::GetDiagnosticsResponse,
    charge_point_state: &mut ChargePointState,
) {
    match &get_diagnostics_response.file_name {
        Some(filename) => {
            info!(
                "GetDiagnostics request with UUID {} has been accepted by the ChargingPoint. Diagnostics report {} will be uploaded!",
                response_uuid, filename
            );
        }
        None => {
            warn!(
                "GetDiagnostics request with UUID {} has been rejected by the ChargingPoint",
                response_uuid
            );
        }
    }

    charge_point_state
        .requests_awaiting_confirmation
        .retain(|e| *e.uuid != *response_uuid);
}
