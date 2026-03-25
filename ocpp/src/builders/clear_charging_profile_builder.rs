use crate::ocpp_types::{CustomError, MessageTypeName};

use super::MessageBuilder;

use rust_ocpp::v1_6::messages;
use rust_ocpp::v1_6::types::ChargingProfilePurposeType;

//-------------------------------------------------------------------------------------------------

pub struct ClearChargingProfileBuilder {
    id: Option<i32>,
    connector_id: Option<i32>,
    charging_profile_purpose: Option<ChargingProfilePurposeType>,
    stack_level: Option<i32>,

    message_type_name: MessageTypeName,
    clear_charging_profile_request:
        Option<messages::clear_charging_profile::ClearChargingProfileRequest>,
}

impl ClearChargingProfileBuilder {
    pub fn default() -> Self {
        Self {
            id: None,
            connector_id: None,
            charging_profile_purpose: None,
            stack_level: None,
            message_type_name: MessageTypeName::ClearChargingProfile,
            clear_charging_profile_request: None,
        }
    }

    pub fn new(
        id: Option<i32>,
        connector_id: Option<i32>,
        charging_profile_purpose: Option<ChargingProfilePurposeType>,
        stack_level: Option<i32>,
    ) -> Self {
        Self {
            id,
            connector_id,
            charging_profile_purpose,
            stack_level,
            message_type_name: MessageTypeName::ClearChargingProfile,
            clear_charging_profile_request: None,
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl MessageBuilder<messages::clear_charging_profile::ClearChargingProfileRequest>
    for ClearChargingProfileBuilder
{
    fn get_message_type_name(&self) -> MessageTypeName {
        self.message_type_name.clone()
    }

    fn get_message_request(
        &self,
    ) -> Result<messages::clear_charging_profile::ClearChargingProfileRequest, CustomError> {
        self.clear_charging_profile_request
            .clone()
            .ok_or(CustomError::Common(
                ".build() has not been called!".to_owned(),
            ))
    }

    fn build(
        mut self,
    ) -> impl MessageBuilder<messages::clear_charging_profile::ClearChargingProfileRequest> {
        self.clear_charging_profile_request = Some(
            messages::clear_charging_profile::ClearChargingProfileRequest {
                id: self.id,
                connector_id: self.connector_id,
                charging_profile_purpose: self.charging_profile_purpose.clone(),
                stack_level: self.stack_level,
            },
        );

        self
    }
}
