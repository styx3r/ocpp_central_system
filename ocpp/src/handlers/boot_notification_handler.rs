use crate::ocpp_types::CustomError;
use config::config;

use rust_ocpp::v1_6::messages::boot_notification;
use rust_ocpp::v1_6::types::RegistrationStatus;

//-------------------------------------------------------------------------------------------------

pub(crate) fn handle_boot_notification_request(
    boot_notification_request: &boot_notification::BootNotificationRequest,
    charging_point_config: &config::ChargePoint,
) -> Result<boot_notification::BootNotificationResponse, CustomError> {
    let charge_point_serial_number = boot_notification_request
        .charge_point_serial_number
        .clone()
        .ok_or(CustomError::Common(
            "ChargePoint did not send serial number!".to_owned(),
        ))?;

    if charge_point_serial_number != charging_point_config.serial_number {
        return Err(CustomError::Common(format!(
            "ChargePoint with serial number {} is not configured!",
            charge_point_serial_number
        )));
    }

    Ok(boot_notification::BootNotificationResponse {
        current_time: chrono::offset::Utc::now(),
        interval: charging_point_config.heartbeat_interval,
        status: RegistrationStatus::Accepted,
    })
}

//-------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use uom::si::{f64::*, power::watt, electric_potential::volt, electric_current::ampere};

    static UNITTEST_CHARGING_POINT_MODEL: &str = "MODEL";
    static UNITTEST_CHARGE_POINT_VENDOR: &str = "VENDOR";
    static UNITTEST_CHARGING_POINT_SERIAL: &str = "SERIAL_NUMBER";

    static UNITTEST_HEARTBEAT_INTERVAL: u32 = 60;
    static UNITTEST_MAX_CHARGING_POWER: f64 = 11000.0;
    static UNITTEST_SYSTEM_VOLTAGE: f64 = 400.0;
    static UNITTEST_DEFAULT_CURRENT: f64 = 16.0;
    static UNITTEST_COS_PHI: f64 = 0.86;
    static UNITTEST_MINIMUM_CHARGING_CURRENT: f64 = 6.0;

    fn default_charge_point_config() -> config::ChargePoint {
        config::ChargePoint {
            serial_number: UNITTEST_CHARGING_POINT_SERIAL.to_owned(),
            heartbeat_interval: UNITTEST_HEARTBEAT_INTERVAL,
            max_charging_power: Power::new::<watt>(UNITTEST_MAX_CHARGING_POWER),
            default_system_voltage: ElectricPotential::new::<volt>(UNITTEST_SYSTEM_VOLTAGE),
            default_current: ElectricCurrent::new::<ampere>(UNITTEST_DEFAULT_CURRENT),
            default_cos_phi: UNITTEST_COS_PHI,
            minimum_charging_current: ElectricCurrent::new::<ampere>(UNITTEST_MINIMUM_CHARGING_CURRENT),
            config_parameters: vec![],
        }
    }

    #[test]
    fn boot_notification_with_empty_serial_number() -> Result<(), CustomError> {
        let response = handle_boot_notification_request(
            &boot_notification::BootNotificationRequest {
                charge_box_serial_number: None,
                charge_point_model: UNITTEST_CHARGING_POINT_MODEL.to_owned(),
                charge_point_serial_number: None,
                charge_point_vendor: UNITTEST_CHARGE_POINT_VENDOR.to_owned(),
                firmware_version: None,
                iccid: None,
                imsi: None,
                meter_serial_number: None,
                meter_type: None,
            },
            &default_charge_point_config(),
        );

        assert!(response.is_err());
        Ok(())
    }

    #[test]
    fn boot_notification_with_invalid_serial_number() -> Result<(), CustomError> {
        let response = handle_boot_notification_request(
            &boot_notification::BootNotificationRequest {
                charge_box_serial_number: None,
                charge_point_model: UNITTEST_CHARGING_POINT_MODEL.to_owned(),
                charge_point_serial_number: Some("INVALID_SERIAL".to_owned()),
                charge_point_vendor: UNITTEST_CHARGE_POINT_VENDOR.to_owned(),
                firmware_version: None,
                iccid: None,
                imsi: None,
                meter_serial_number: None,
                meter_type: None,
            },
            &default_charge_point_config(),
        );

        assert!(response.is_err());
        Ok(())
    }

    #[test]
    fn boot_notification_with_valid_serial_number() -> Result<(), CustomError> {
        let response = handle_boot_notification_request(
            &boot_notification::BootNotificationRequest {
                charge_box_serial_number: None,
                charge_point_model: UNITTEST_CHARGING_POINT_MODEL.to_owned(),
                charge_point_serial_number: Some(UNITTEST_CHARGING_POINT_SERIAL.to_owned()),
                charge_point_vendor: UNITTEST_CHARGE_POINT_VENDOR.to_owned(),
                firmware_version: None,
                iccid: None,
                imsi: None,
                meter_serial_number: None,
                meter_type: None,
            },
            &default_charge_point_config(),
        )?;

        assert_eq!(response.interval, UNITTEST_HEARTBEAT_INTERVAL);
        assert_eq!(response.status, RegistrationStatus::Accepted);

        Ok(())
    }
}
