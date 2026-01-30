use crate::ocpp_types::{CustomError, MessageTypeName};

use crate::builders::MessageBuilder;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_ocpp::v1_6::messages;
use rust_ocpp::v1_6::types::{
    ChargingProfile, ChargingProfileKindType, ChargingProfilePurposeType, ChargingRateUnitType,
    ChargingSchedule, ChargingSchedulePeriod, RecurrencyKindType,
};

//-------------------------------------------------------------------------------------------------

pub struct ChargingProfileBuilder {
    charging_profile: ChargingProfile,
}

impl ChargingProfileBuilder {
    pub fn new(
        charging_profile_id: i32,
        charging_profile_purpose: ChargingProfilePurposeType,
        charging_profile_kind: ChargingProfileKindType,
        charging_rate_unit: ChargingRateUnitType,
    ) -> Self {
        Self {
            charging_profile: ChargingProfile {
                charging_profile_id,
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
                    charging_rate_unit,
                    charging_schedule_period: vec![],
                    min_charging_rate: None,
                },
            },
        }
    }

    pub fn set_transaction_id(mut self, transaction_id: i32) -> ChargingProfileBuilder {
        self.charging_profile.transaction_id = Some(transaction_id);
        self
    }

    pub fn set_recurrency_kind(mut self, recurrency_kind: RecurrencyKindType) -> ChargingProfileBuilder {
        self.charging_profile.recurrency_kind = Some(recurrency_kind);
        self
    }

    pub fn set_valid_from(mut self, valid_from: DateTime<Utc>) -> ChargingProfileBuilder {
        self.charging_profile.valid_from = Some(valid_from);
        self
    }

    pub fn set_valid_to(mut self, valid_to: DateTime<Utc>) -> ChargingProfileBuilder {
        self.charging_profile.valid_to = Some(valid_to);
        self
    }

    pub fn set_charging_schedule_duration(mut self, duration: i32) -> ChargingProfileBuilder {
        self.charging_profile.charging_schedule.duration = Some(duration);
        self
    }

    pub fn set_start_schedule_timestamp(mut self, start_timestamp: DateTime<Utc>) -> ChargingProfileBuilder {
        self.charging_profile.charging_schedule.start_schedule = Some(start_timestamp);
        self
    }

    pub fn set_schedule_min_charging_rate(mut self, min_charging_rate: Decimal) -> ChargingProfileBuilder {
        self.charging_profile.charging_schedule.min_charging_rate = Some(min_charging_rate);
        self
    }

    pub fn add_charging_schedule_period(
        mut self,
        start_period: i32,
        limit: Decimal,
        number_phases: Option<i32>,
    ) -> ChargingProfileBuilder {
        self.charging_profile
            .charging_schedule
            .charging_schedule_period
            .push(ChargingSchedulePeriod {
                start_period,
                limit,
                number_phases,
            });

        self
    }

    pub fn get(self) -> ChargingProfile {
        return self.charging_profile.clone();
    }
}
