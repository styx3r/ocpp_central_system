use rust_ocpp::v1_6::messages::status_notification;

use crate::ocpp::ocpp_types::CustomError;
use log::info;

//------------------------------------------------------------------------------------------------

pub(crate) fn handle_status_notification_request(
    status_notification: &status_notification::StatusNotificationRequest,
) -> Result<status_notification::StatusNotificationResponse, CustomError> {
    info!(
        "Received StatusNotificationRequest with context: {:?}",
        status_notification
    );

    Ok(status_notification::StatusNotificationResponse {})
}

//------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    static UNITTEST_CONNECTOR_ID: u32 = 1;

    #[test]
    fn status_notification() -> Result<(), CustomError> {
        let response =
            handle_status_notification_request(&status_notification::StatusNotificationRequest {
                connector_id: UNITTEST_CONNECTOR_ID,
                error_code: rust_ocpp::v1_6::types::ChargePointErrorCode::NoError,
                info: None,
                status: rust_ocpp::v1_6::types::ChargePointStatus::Available,
                timestamp: None,
                vendor_id: None,
                vendor_error_code: None,
            })?;

        assert_eq!(response, status_notification::StatusNotificationResponse {});

        Ok(())
    }
}
