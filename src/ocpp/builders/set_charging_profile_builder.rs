use crate::ocpp::ocpp_types::{CustomError, MessageTypeName};

use crate::ocpp::builders::MessageBuilder;

use rust_decimal::Decimal;
use rust_ocpp::v1_6::messages;
use rust_ocpp::v1_6::types::{
    ChargingProfile, ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
    ChargingSchedule, ChargingSchedulePeriod,
};

pub(crate) struct SetChargingProfileBuilder {
    connector_id: i32,
    charging_profile: ChargingProfile,

    message_type_name: MessageTypeName,
    set_charging_profile_request: Option<messages::set_charging_profile::SetChargingProfileRequest>,
}

// TODO(styx3r): Check if some setter functions make the usage cleaner
impl SetChargingProfileBuilder {
    pub(crate) fn new(
        connector_id: i32,
        charging_profile_id: i32,
        charging_profile_purpose: ChargingProfilePurposeType,
        charging_profile_kind: ChargingProfileKindType,
        charging_rate_unit: ChargingRateUnitType,
    ) -> Self {
        Self {
            connector_id,
            charging_profile: ChargingProfile {
                charging_profile_id: charging_profile_id,
                transaction_id: None,
                stack_level: 0,
                charging_profile_purpose,
                charging_profile_kind,
                recurrency_kind: None,
                valid_from: None,
                valid_to: None,
                charging_schedule: ChargingSchedule {
                    duration: None,
                    start_schedule: None,
                    charging_rate_unit: charging_rate_unit,
                    charging_schedule_period: vec![],
                    min_charging_rate: None,
                },
            },
            message_type_name: MessageTypeName::SetChargingProfile,
            set_charging_profile_request: None,
        }
    }

    pub(crate) fn add_charging_schedule_period(
        &mut self,
        start_period: i32,
        limit: &Decimal,
        number_phases: Option<i32>,
    ) -> &mut Self {
        self.charging_profile
            .charging_schedule
            .charging_schedule_period
            .push(ChargingSchedulePeriod {
                start_period,
                limit: limit.to_owned(),
                number_phases: number_phases.to_owned(),
            });

        self
    }
}

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

    fn build(&mut self) -> &mut Self {
        self.set_charging_profile_request =
            Some(messages::set_charging_profile::SetChargingProfileRequest {
                connector_id: self.connector_id,
                cs_charging_profiles: self.charging_profile.to_owned(),
            });

        self
    }
}
