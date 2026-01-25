use crate::ocpp_types::{CustomError, MessageTypeName};

use rust_ocpp::v1_6::messages::firmware_status_notification;

use log::info;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_firmware_status_notification_request(
    firmware_status_notification_request: &firmware_status_notification::FirmwareStatusNotificationRequest,
) -> Result<firmware_status_notification::FirmwareStatusNotificationResponse, CustomError> {
    info!(
        "Received {} with content: {:?}",
        MessageTypeName::FirmwareStatusNotification,
        firmware_status_notification_request
    );

    Ok(firmware_status_notification::FirmwareStatusNotificationResponse {})
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firmware_status_notification() -> Result<(), CustomError> {
        let response = handle_firmware_status_notification_request(
            &firmware_status_notification::FirmwareStatusNotificationRequest {
                status: rust_ocpp::v1_6::types::FirmwareStatus::Installed,
            },
        )?;

        assert_eq!(
            response,
            firmware_status_notification::FirmwareStatusNotificationResponse {}
        );

        Ok(())
    }
}
