use crate::ocpp_types::{CustomError, MessageTypeName};

use log::warn;
use rust_ocpp::v2_0_1::messages::security_event_notification;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_security_event_notification_request(
    security_event_notification_request: &security_event_notification::SecurityEventNotificationRequest,
) -> Result<security_event_notification::SecurityEventNotificationResponse, CustomError> {
    warn!(
        "Received {} with context: {:?}",
        MessageTypeName::SecurityEventNotification,
        security_event_notification_request
    );

    Ok(security_event_notification::SecurityEventNotificationResponse {})
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    static UNITTEST_SECURITY_NOTIFICATION_REQUEST_KIND: &str = "UNITTEST";

    #[test]
    fn security_event_notification_request() -> Result<(), CustomError> {
        let response = handle_security_event_notification_request(
            &security_event_notification::SecurityEventNotificationRequest {
                kind: UNITTEST_SECURITY_NOTIFICATION_REQUEST_KIND.to_string(),
                timestamp: chrono::offset::Utc::now(),
                tech_info: None,
            },
        )?;

        assert_eq!(
            response,
            security_event_notification::SecurityEventNotificationResponse {}
        );

        Ok(())
    }
}
