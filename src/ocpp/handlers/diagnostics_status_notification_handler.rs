use crate::ocpp::ocpp_types::{CustomError, MessageTypeName};

use rust_ocpp::v1_6::messages::diagnostics_status_notification;

use log::info;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_diagnostic_status_notification_request(
    diagnostic_status_notification_request: &diagnostics_status_notification::DiagnosticsStatusNotificationRequest,
) -> Result<diagnostics_status_notification::DiagnosticsStatusNotificationResponse, CustomError> {
    info!(
        "Received {} with content: {:?}",
        MessageTypeName::DiagnosticsStatusNotification,
        diagnostic_status_notification_request
    );

    Ok(diagnostics_status_notification::DiagnosticsStatusNotificationResponse {})
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_status_notification() -> Result<(), CustomError> {
        let response = handle_diagnostic_status_notification_request(
            &diagnostics_status_notification::DiagnosticsStatusNotificationRequest {
                status: rust_ocpp::v1_6::types::DiagnosticsStatus::Idle,
            },
        )?;

        assert_eq!(
            response,
            diagnostics_status_notification::DiagnosticsStatusNotificationResponse {}
        );

        Ok(())
    }
}
