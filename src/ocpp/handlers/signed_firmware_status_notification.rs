use crate::ocpp::ocpp_types::{CustomError, MessageTypeName};

use log::warn;
use rust_ocpp::v1_6::messages::firmware_status_notification;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_signed_firmware_status_notification_request(
    signed_firmware_status_notification_request: &firmware_status_notification::FirmwareStatusNotificationRequest,
) -> Result<firmware_status_notification::FirmwareStatusNotificationResponse, CustomError> {
    warn!(
        "Received {} with context: {:?}",
        MessageTypeName::FirmwareStatusNotification,
        signed_firmware_status_notification_request
    );

    Ok(firmware_status_notification::FirmwareStatusNotificationResponse {})
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_firmware_status_notification_request() -> Result<(), CustomError> {
        let response = handle_signed_firmware_status_notification_request(
            &firmware_status_notification::FirmwareStatusNotificationRequest {
                status: rust_ocpp::v1_6::types::FirmwareStatus::Idle,
            },
        )?;

        assert_eq!(
            response,
            firmware_status_notification::FirmwareStatusNotificationResponse {}
        );

        Ok(())
    }
}
