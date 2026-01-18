use crate::config::IdTag;
use crate::ocpp::ocpp_types::CustomError;

use rust_ocpp::v1_6::messages::authorize;
use rust_ocpp::v1_6::types::{AuthorizationStatus, IdTagInfo};

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_authorize_request(
    authorize_request: &authorize::AuthorizeRequest,
    authorized_id_tags: &Vec<IdTag>,
) -> Result<authorize::AuthorizeResponse, CustomError> {
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

    #[test]
    fn authorize_without_authorized_id_tags() -> Result<(), CustomError> {
        let response = handle_authorize_request(
            &authorize::AuthorizeRequest {
                id_tag: UNITTEST_ID_TAG.to_owned(),
            },
            &vec![],
        )?;

        assert_eq!(response.id_tag_info.status, AuthorizationStatus::Blocked);
        Ok(())
    }

    #[test]
    fn authorize_with_authorized_id_tags() -> Result<(), CustomError> {
        let response = handle_authorize_request(
            &authorize::AuthorizeRequest {
                id_tag: UNITTEST_ID_TAG.to_owned(),
            },
            &vec![IdTag {
                id: UNITTEST_ID_TAG.to_owned(),
            }],
        )?;

        assert_eq!(response.id_tag_info.status, AuthorizationStatus::Accepted);
        Ok(())
    }
}
