use rust_ocpp::v1_6::{messages::status_notification, types::ChargePointStatus};

use crate::{OcppStatusNotificationHook, ocpp_types::CustomError};
use log::{info, error};

use std::sync::{Arc, Mutex};

//------------------------------------------------------------------------------------------------

pub(crate) fn handle_status_notification_request<T: OcppStatusNotificationHook>(
    status_notification: &status_notification::StatusNotificationRequest,
    hook: Arc<Mutex<T>>
) -> Result<status_notification::StatusNotificationResponse, CustomError> {
    info!(
        "Received StatusNotificationRequest with context: {:?}",
        status_notification
    );

    match hook.lock().unwrap().evaluate(status_notification) {
        Err(_) => error!("Hook failed!"),
        _ => {}
    }

    Ok(status_notification::StatusNotificationResponse {})
}

//------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    static UNITTEST_CONNECTOR_ID: u32 = 1;

    struct Hook {
        pub called: bool
    }

    impl Hook {
        pub fn default() -> Self {
            Self { called: false }
        }
    }

    impl OcppStatusNotificationHook for Hook {
        fn evaluate(
                &mut self,
                _status_notification: &status_notification::StatusNotificationRequest,
            ) -> Result<(), Box<dyn std::error::Error>> {
            self.called = true;

            Ok(())
        }
    }

    #[test]
    fn status_notification() -> Result<(), CustomError> {
        let hook = Arc::new(Mutex::new(Hook::default()));
        let response =
            handle_status_notification_request(&status_notification::StatusNotificationRequest {
                connector_id: UNITTEST_CONNECTOR_ID,
                error_code: rust_ocpp::v1_6::types::ChargePointErrorCode::NoError,
                info: None,
                status: rust_ocpp::v1_6::types::ChargePointStatus::Available,
                timestamp: None,
                vendor_id: None,
                vendor_error_code: None,
            },
            Arc::clone(&hook))?;

        assert!(hook.lock().unwrap().called);
        assert_eq!(response, status_notification::StatusNotificationResponse {});

        Ok(())
    }
}
