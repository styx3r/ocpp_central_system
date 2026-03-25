use chrono::Utc;
use ocpp::{
    AuthorizeRequest, ElectricCurrent, ElectricPotential, Energy, Frequency, Measurand,
    MultiPhaseMeasurand, Phase, Power, StatusNotificationRequest, TemperatureInterval, Unit,
};
use rusqlite::Connection;

pub(crate) struct Persistence {}

impl Persistence {
    pub fn setup(db_connection: &Connection) -> Result<usize, rusqlite::Error> {
        // AuthorizeRequest
        db_connection.execute(
            "CREATE TABLE IF NOT EXISTS authorize_requests (id INT PRIMARY KEY, id_tag TEXT);",
            (),
        )?;

        // StatusNotification
        db_connection.execute(
            "CREATE TABLE IF NOT EXISTS status_notifications (id INT PRIMARY KEY, connector_id INT, error_code TEXT, info TEXT, status TEXT, timestamp INT, vendor_id TEXT, vendor_error_code TEXT);",
            ()
        )?;

        // MeterReadings
        db_connection.execute(
            "CREATE TABLE IF NOT EXISTS meter_readings (id INT PRIMARY KEY, name TEXT, timestamp INT, value REAL, unit TEXT, phase TEXT);",
            ()
        )
    }

    pub fn store_authorize_request(
        db_connection: &Connection,
        authorize_request: &AuthorizeRequest,
    ) -> Result<usize, rusqlite::Error> {
        db_connection.execute(
            "INSERT INTO authorize_requests (id_tag) VALUES (?1);",
            [authorize_request.id_tag.clone()],
        )
    }

    pub fn store_meter_readings(
        db_connection: &Connection,
        measurand: &Measurand,
    ) -> Result<usize, rusqlite::Error> {
        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "CurrentExport",
            &measurand.current_export,
        )?;
        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "CurrentImport",
            &measurand.current_import,
        )?;

        if let Some(current_offered) = measurand.current_offered {
            MeterReadingsStorer::store_phase(db_connection, "CurrentOffered", &current_offered)?;
        }

        if let Some(energy_active_export_register) = measurand.energy_active_export_register {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyActiveExportRegister",
                &energy_active_export_register,
            )?;
        }

        if let Some(energy_active_import_register) = measurand.energy_active_import_register {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyActiveImportRegister",
                &energy_active_import_register,
            )?;
        }

        if let Some(energy_reactive_export_register) = measurand.energy_reactive_export_register {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyReactiveExportRegister",
                &energy_reactive_export_register,
            )?;
        }

        if let Some(energy_reactive_import_register) = measurand.energy_reactive_import_register {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyReactiveImportRegister",
                &energy_reactive_import_register,
            )?;
        }

        if let Some(energy_active_export_interval) = measurand.energy_active_export_interval {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyActiveExportInterval",
                &energy_active_export_interval,
            )?;
        }

        if let Some(energy_active_import_interval) = measurand.energy_active_import_interval {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyActiveImportInterval",
                &energy_active_import_interval,
            )?;
        }

        if let Some(energy_reactive_export_interval) = measurand.energy_reactive_export_interval {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyReactiveExportInterval",
                &energy_reactive_export_interval,
            )?;
        }

        if let Some(energy_reactive_import_interval) = measurand.energy_reactive_import_interval {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyReactiveImportInterval",
                &energy_reactive_import_interval,
            )?;
        }

        if let Some(frequency) = measurand.frequency {
            MeterReadingsStorer::store_phase(db_connection, "Frequency", &frequency)?;
        }

        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "PowerActiveExport",
            &measurand.power_active_export,
        )?;
        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "PowerActiveImport",
            &measurand.power_active_import,
        )?;

        if let Some(power_factor) = measurand.power_factor {
            MeterReadingsStorer::store_phase(db_connection, "PowerFactor", &power_factor)?;
        }

        if let Some(power_offered) = measurand.power_offered {
            MeterReadingsStorer::store_phase(db_connection, "PowerOffered", &power_offered)?;
        }

        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "PowerReactiveExport",
            &measurand.power_reactive_export,
        )?;
        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "PowerReactiveImport",
            &measurand.power_reactive_import,
        )?;

        if let Some(rpm) = measurand.rpm {
            MeterReadingsStorer::store_phase(db_connection, "RPM", &rpm)?;
        }

        if let Some(state_of_charge) = measurand.state_of_charge {
            MeterReadingsStorer::store_phase(db_connection, "SoC", &state_of_charge)?;
        }

        if let Some(temperature) = measurand.temperature {
            MeterReadingsStorer::store_phase(db_connection, "Temperature", &temperature)?;
        }

        MeterReadingsStorer::store_multi_phase(db_connection, "Voltage", &measurand.voltage)?;

        Ok(0)
    }

    pub fn store_status_notification(
        db_connection: &Connection,
        status_notification: &StatusNotificationRequest,
    ) -> Result<usize, rusqlite::Error> {
        db_connection.execute(
            "INSERT INTO status_notifications (connector_id, error_code, info, status, timestamp, vendor_id, vendor_error_code) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            (status_notification.connector_id,
            serde_json::to_string(&status_notification.error_code).unwrap_or(String::default()),
            status_notification.info.clone().unwrap_or(String::default()),
            format!("{:?}", status_notification.status),
            status_notification.timestamp.unwrap_or(Utc::now()).timestamp_millis(),
            status_notification.vendor_id.clone().unwrap_or(String::default()),
            status_notification.vendor_error_code.clone().unwrap_or(String::default()))
        )
    }
}

struct MeterReadingsStorer {}

trait StorePhaseReading<T> {
    fn store_multi_phase(
        db_connection: &Connection,
        name: &str,
        meter_readings: &MultiPhaseMeasurand<T>,
    ) -> Result<usize, rusqlite::Error>;

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        meter_reading: &T,
    ) -> Result<usize, rusqlite::Error>;
}

impl StorePhaseReading<ElectricCurrent> for MeterReadingsStorer {
    fn store_multi_phase(
        db_connection: &Connection,
        name: &str,
        multi_phase_measurement: &MultiPhaseMeasurand<ElectricCurrent>,
    ) -> Result<usize, rusqlite::Error> {
        let now = Utc::now().timestamp_millis();
        let mut inserted_rows = 0;

        for phase in [Phase::L1, Phase::L2, Phase::L3] {
            match multi_phase_measurement.get_phase(phase) {
                Some(phase_measurand) => {
                    inserted_rows += db_connection.execute(
                        "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
                        (
                            name,
                            now,
                            serde_json::to_string(&phase_measurand.value)
                                .unwrap_or(String::default()),
                            ocpp::ampere::abbreviation(),
                            phase_measurand.phase.to_string(),
                        ),
                    )?;
                }
                _ => {}
            }
        }

        Ok(inserted_rows)
    }

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        meter_reading: &ElectricCurrent,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                Utc::now().timestamp_millis(),
                serde_json::to_string(meter_reading).unwrap_or(String::new()),
                ocpp::ampere::abbreviation(),
                String::default(),
            ),
        );
    }
}

impl StorePhaseReading<Energy> for MeterReadingsStorer {
    fn store_multi_phase(
        db_connection: &Connection,
        name: &str,
        multi_phase_measurement: &MultiPhaseMeasurand<Energy>,
    ) -> Result<usize, rusqlite::Error> {
        let now = Utc::now().timestamp_millis();
        let mut inserted_rows = 0;

        for phase in [Phase::L1, Phase::L2, Phase::L3] {
            match multi_phase_measurement.get_phase(phase) {
                Some(phase_measurand) => {
                    inserted_rows += db_connection.execute(
                        "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
                        (
                            name,
                            now,
                            serde_json::to_string(&phase_measurand.value)
                                .unwrap_or(String::default()),
                            ocpp::watt_hour::abbreviation(),
                            phase_measurand.phase.to_string(),
                        ),
                    )?;
                }
                _ => {}
            }
        }

        Ok(inserted_rows)
    }

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        meter_reading: &Energy,
    ) -> Result<usize, rusqlite::Error> {
        let now = Utc::now().timestamp_millis();
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                now,
                serde_json::to_string(meter_reading).unwrap_or(String::default()),
                ocpp::watt_hour::abbreviation(),
                String::default(),
            ),
        );
    }
}

impl StorePhaseReading<Frequency> for MeterReadingsStorer {
    fn store_multi_phase(
        _db_connection: &Connection,
        _name: &str,
        _multi_phase_measurement: &MultiPhaseMeasurand<Frequency>,
    ) -> Result<usize, rusqlite::Error> {
        Ok(0)
    }

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        meter_reading: &Frequency,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                Utc::now().timestamp_millis(),
                serde_json::to_string(meter_reading).unwrap_or(String::default()),
                ocpp::hertz::abbreviation(),
                String::default(),
            ),
        );
    }
}

impl StorePhaseReading<Power> for MeterReadingsStorer {
    fn store_multi_phase(
        db_connection: &Connection,
        name: &str,
        multi_phase_measurement: &MultiPhaseMeasurand<Power>,
    ) -> Result<usize, rusqlite::Error> {
        let now = Utc::now().timestamp_millis();
        let mut inserted_rows = 0;

        for phase in [Phase::L1, Phase::L2, Phase::L3] {
            match multi_phase_measurement.get_phase(phase) {
                Some(phase_measurand) => {
                    inserted_rows += db_connection.execute(
                        "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
                        (
                            name,
                            now,
                            serde_json::to_string(&phase_measurand.value)
                                .unwrap_or(String::default()),
                            ocpp::watt::abbreviation(),
                            phase_measurand.phase.to_string(),
                        ),
                    )?;
                }
                _ => {}
            }
        }

        Ok(inserted_rows)
    }

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        meter_reading: &Power,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                Utc::now().timestamp_millis(),
                serde_json::to_string(meter_reading).unwrap_or(String::default()),
                ocpp::watt::abbreviation(),
                String::default(),
            ),
        );
    }
}

impl StorePhaseReading<TemperatureInterval> for MeterReadingsStorer {
    fn store_multi_phase(
        _db_connection: &Connection,
        _name: &str,
        _multi_phase_measurement: &MultiPhaseMeasurand<TemperatureInterval>,
    ) -> Result<usize, rusqlite::Error> {
        Ok(0)
    }

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        meter_reading: &TemperatureInterval,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                Utc::now().timestamp_millis(),
                serde_json::to_string(meter_reading).unwrap_or(String::default()),
                ocpp::degree_celsius::abbreviation(),
                String::default(),
            ),
        );
    }
}

impl StorePhaseReading<ElectricPotential> for MeterReadingsStorer {
    fn store_multi_phase(
        db_connection: &Connection,
        name: &str,
        multi_phase_measurement: &MultiPhaseMeasurand<ElectricPotential>,
    ) -> Result<usize, rusqlite::Error> {
        let now = Utc::now().timestamp_millis();
        let mut inserted_rows = 0;

        for phase in [Phase::L1, Phase::L2, Phase::L3] {
            match multi_phase_measurement.get_phase(phase.clone()) {
                Some(phase_measurand) => {
                    inserted_rows += db_connection.execute(
                        "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
                        (
                            name,
                            now,
                            serde_json::to_string(&phase_measurand.value)
                                .unwrap_or(String::default()),
                            ocpp::volt::abbreviation(),
                            phase_measurand.phase.to_string(),
                        ),
                    )?;
                }
                _ => {}
            }
        }

        Ok(inserted_rows)
    }

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        meter_reading: &ElectricPotential,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                Utc::now().timestamp_millis(),
                serde_json::to_string(meter_reading).unwrap_or(String::default()),
                ocpp::volt::abbreviation(),
                String::default(),
            ),
        );
    }
}

impl StorePhaseReading<f64> for MeterReadingsStorer {
    fn store_multi_phase(
        _db_connection: &Connection,
        _name: &str,
        _multi_phase_measurement: &MultiPhaseMeasurand<f64>,
    ) -> Result<usize, rusqlite::Error> {
        Ok(0)
    }

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        meter_reading: &f64,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (name, Utc::now().timestamp_millis(), meter_reading, String::default(), String::default()),
        );
    }
}
