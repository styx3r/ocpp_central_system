use crate::ocpp::ocpp_types::CustomError;

use rust_ocpp::v1_6::messages::heart_beat;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_heartbeat_request() -> Result<heart_beat::HeartbeatResponse, CustomError> {
    Ok(heart_beat::HeartbeatResponse {
        current_time: chrono::offset::Utc::now(),
    })
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_request() -> Result<(), CustomError> {
        let _ = handle_heartbeat_request()?;
        Ok(())
    }
}
