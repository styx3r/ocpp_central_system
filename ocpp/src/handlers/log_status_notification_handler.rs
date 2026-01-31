use crate::ocpp_types::{CustomError, MessageTypeName};

use log::warn;
use rust_ocpp::v2_0_1::messages::log_status_notification;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_log_status_notification_request(
    log_status_notification_request: &log_status_notification::LogStatusNotificationRequest,
) -> Result<log_status_notification::LogStatusNotificationResponse, CustomError> {
    warn!(
        "Received {} with context: {:?}",
        MessageTypeName::LogStatusNotification,
        log_status_notification_request.status
    );

    Ok(log_status_notification::LogStatusNotificationResponse {})
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn log_status_request() -> Result<(), CustomError> {
        let response = handle_log_status_notification_request(
            &log_status_notification::LogStatusNotificationRequest {
                status: rust_ocpp::v2_0_1::enumerations::upload_log_status_enum_type::UploadLogStatusEnumType::Idle,
                request_id: None
            }
        )?;

        assert_eq!(
            response,
            log_status_notification::LogStatusNotificationResponse {}
        );

        Ok(())
    }
}
