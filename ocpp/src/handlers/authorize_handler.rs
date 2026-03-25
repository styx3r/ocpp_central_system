use crate::ocpp_types::CustomError;
use crate::{ChargePointState, OcppAuthorizationHook};
use config::config::IdTag;
use std::sync::{Arc, Mutex};

use log::error;

use rust_ocpp::v1_6::messages::authorize;
use rust_ocpp::v1_6::types::{AuthorizationStatus, IdTagInfo};

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_authorize_request<T: OcppAuthorizationHook>(
    authorize_request: &authorize::AuthorizeRequest,
    authorized_id_tags: &Vec<IdTag>,
    charge_point_state: &mut ChargePointState,
    hook: Arc<Mutex<T>>,
) -> Result<authorize::AuthorizeResponse, CustomError> {
    match hook
        .lock()
        .unwrap()
        .evaluate(authorize_request, charge_point_state)
    {
        Err(err) => error!("Hook failed: {}", err),
        _ => {}
    }

    Ok(authorize::AuthorizeResponse {
        id_tag_info: IdTagInfo {
            expiry_date: None,
            parent_id_tag: None,
            status: match authorized_id_tags
                .iter()
                .find(|e| e.id == authorize_request.id_tag)
            {
                Some(_) => AuthorizationStatus::Accepted,
                None => AuthorizationStatus::Blocked,
            },
        },
    })
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    static UNITTEST_ID_TAG: &str = "UNITTEST_ID_TAG";

    struct Hook {
        pub called: bool,
    }

    impl Hook {
        pub fn default() -> Self {
            Self { called: false }
        }
    }

    impl OcppAuthorizationHook for Hook {
        fn evaluate(
            &mut self,
            _authorize_request: &authorize::AuthorizeRequest,
            _charge_point_state: &mut ChargePointState,
        ) -> Result<(), Box<dyn std::error::Error>> {
            self.called = true;
            Ok(())
        }
    }

    #[test]
    fn authorize_without_authorized_id_tags() -> Result<(), CustomError> {
        let mut charge_point_state = ChargePointState::default();
        let hook = Arc::new(Mutex::new(Hook::default()));

        let response = handle_authorize_request(
            &authorize::AuthorizeRequest {
                id_tag: UNITTEST_ID_TAG.to_owned(),
            },
            &vec![],
            &mut charge_point_state,
            Arc::clone(&hook),
        )?;

        assert_eq!(response.id_tag_info.status, AuthorizationStatus::Blocked);
        Ok(())
    }

    #[test]
    fn authorize_with_authorized_id_tags() -> Result<(), CustomError> {
        let mut charge_point_state = ChargePointState::default();
        let hook = Arc::new(Mutex::new(Hook::default()));

        let response = handle_authorize_request(
            &authorize::AuthorizeRequest {
                id_tag: UNITTEST_ID_TAG.to_owned(),
            },
            &vec![IdTag {
                id: UNITTEST_ID_TAG.to_owned(),
                smart_charging_mode:
                    config::config::SmartChargingMode::PVOverProductionAndGridBased,
            }],
            &mut charge_point_state,
            Arc::clone(&hook),
        )?;

        assert_eq!(response.id_tag_info.status, AuthorizationStatus::Accepted);
        Ok(())
    }
}
