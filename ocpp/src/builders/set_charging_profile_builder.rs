use crate::ocpp_types::{CustomError, MessageTypeName};

use crate::builders::MessageBuilder;

use rust_ocpp::v1_6::messages;
use rust_ocpp::v1_6::types::ChargingProfile;

//-------------------------------------------------------------------------------------------------

pub struct SetChargingProfileBuilder {
    connector_id: i32,
    charging_profile: ChargingProfile,

    message_type_name: MessageTypeName,
    set_charging_profile_request: Option<messages::set_charging_profile::SetChargingProfileRequest>,
}

impl SetChargingProfileBuilder {
    pub fn new(connector_id: i32, charging_profile: ChargingProfile) -> Self {
        Self {
            connector_id,
            charging_profile,
            message_type_name: MessageTypeName::SetChargingProfile,
            set_charging_profile_request: None,
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl MessageBuilder<messages::set_charging_profile::SetChargingProfileRequest>
    for SetChargingProfileBuilder
{
    fn get_message_type_name(&self) -> MessageTypeName {
        self.message_type_name.to_owned()
    }

    fn get_message_request(
        &self,
    ) -> Result<messages::set_charging_profile::SetChargingProfileRequest, CustomError> {
        self.set_charging_profile_request
            .clone()
            .ok_or(CustomError::Common(
                ".build() has not been called!".to_owned(),
            ))
    }

    fn build(
        mut self,
    ) -> impl MessageBuilder<messages::set_charging_profile::SetChargingProfileRequest> {
        self.set_charging_profile_request =
            Some(messages::set_charging_profile::SetChargingProfileRequest {
                connector_id: self.connector_id,
                cs_charging_profiles: self.charging_profile.to_owned(),
            });

        self
    }
}
