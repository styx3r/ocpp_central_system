use chrono::{DateTime, Utc};
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
            "CREATE TABLE IF NOT EXISTS authorize_requests (id INT PRIMARY KEY AUTOINCREMENT, timestamp INT, id_tag TEXT);",
            (),
        )?;

        // StatusNotification
        db_connection.execute(
            "CREATE TABLE IF NOT EXISTS status_notifications (id INT PRIMARY KEY AUTOINCREMENT, connector_id INT, error_code TEXT, info TEXT, status TEXT, timestamp INT, vendor_id TEXT, vendor_error_code TEXT);",
            ()
        )?;

        // MeterReadings
        db_connection.execute(
            "CREATE TABLE IF NOT EXISTS meter_readings (id INT PRIMARY KEY AUTOINCREMENT, name TEXT, timestamp INT, value REAL, unit TEXT, phase TEXT);",
            ()
        )
    }

    pub fn store_authorize_request(
        db_connection: &Connection,
        authorize_request: &AuthorizeRequest,
    ) -> Result<usize, rusqlite::Error> {
        db_connection.execute(
            "INSERT INTO authorize_requests (timestamp, id_tag) VALUES (?1, ?2);",
            (
                Utc::now().timestamp_millis(),
                authorize_request.id_tag.clone(),
            ),
        )
    }

    pub fn store_meter_readings(
        db_connection: &Connection,
        measurand: &Measurand,
    ) -> Result<usize, rusqlite::Error> {
        let now = Utc::now();
        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "CurrentExport",
            now,
            &measurand.current_export,
        )?;
        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "CurrentImport",
            now,
            &measurand.current_import,
        )?;

        if let Some(current_offered) = measurand.current_offered {
            MeterReadingsStorer::store_phase(db_connection, "CurrentOffered", now, &current_offered)?;
        }

        if let Some(energy_active_export_register) = measurand.energy_active_export_register {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyActiveExportRegister",
                now,
                &energy_active_export_register,
            )?;
        }

        if let Some(energy_active_import_register) = measurand.energy_active_import_register {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyActiveImportRegister",
                now,
                &energy_active_import_register,
            )?;
        }

        if let Some(energy_reactive_export_register) = measurand.energy_reactive_export_register {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyReactiveExportRegister",
                now,
                &energy_reactive_export_register,
            )?;
        }

        if let Some(energy_reactive_import_register) = measurand.energy_reactive_import_register {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyReactiveImportRegister",
                now,
                &energy_reactive_import_register,
            )?;
        }

        if let Some(energy_active_export_interval) = measurand.energy_active_export_interval {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyActiveExportInterval",
                now,
                &energy_active_export_interval,
            )?;
        }

        if let Some(energy_active_import_interval) = measurand.energy_active_import_interval {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyActiveImportInterval",
                now,
                &energy_active_import_interval,
            )?;
        }

        if let Some(energy_reactive_export_interval) = measurand.energy_reactive_export_interval {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyReactiveExportInterval",
                now,
                &energy_reactive_export_interval,
            )?;
        }

        if let Some(energy_reactive_import_interval) = measurand.energy_reactive_import_interval {
            MeterReadingsStorer::store_phase(
                db_connection,
                "EnergyReactiveImportInterval",
                now,
                &energy_reactive_import_interval,
            )?;
        }

        if let Some(frequency) = measurand.frequency {
            MeterReadingsStorer::store_phase(db_connection, "Frequency", now, &frequency)?;
        }

        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "PowerActiveExport",
            now,
            &measurand.power_active_export,
        )?;
        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "PowerActiveImport",
            now,
            &measurand.power_active_import,
        )?;

        if let Some(power_factor) = measurand.power_factor {
            MeterReadingsStorer::store_phase(db_connection, "PowerFactor", now, &power_factor)?;
        }

        if let Some(power_offered) = measurand.power_offered {
            MeterReadingsStorer::store_phase(db_connection, "PowerOffered", now, &power_offered)?;
        }

        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "PowerReactiveExport",
            now,
            &measurand.power_reactive_export,
        )?;
        MeterReadingsStorer::store_multi_phase(
            db_connection,
            "PowerReactiveImport",
            now,
            &measurand.power_reactive_import,
        )?;

        if let Some(rpm) = measurand.rpm {
            MeterReadingsStorer::store_phase(db_connection, "RPM", now, &rpm)?;
        }

        if let Some(state_of_charge) = measurand.state_of_charge {
            MeterReadingsStorer::store_phase(db_connection, "SoC", now, &state_of_charge)?;
        }

        if let Some(temperature) = measurand.temperature {
            MeterReadingsStorer::store_phase(db_connection, "Temperature", now, &temperature)?;
        }

        MeterReadingsStorer::store_multi_phase(db_connection, "Voltage", now, &measurand.voltage)?;

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
        timestamp: DateTime<Utc>,
        meter_readings: &MultiPhaseMeasurand<T>,
    ) -> Result<usize, rusqlite::Error>;

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        timestamp: DateTime<Utc>,
        meter_reading: &T,
    ) -> Result<usize, rusqlite::Error>;
}

impl StorePhaseReading<ElectricCurrent> for MeterReadingsStorer {
    fn store_multi_phase(
        db_connection: &Connection,
        name: &str,
        timestamp: DateTime<Utc>,
        multi_phase_measurement: &MultiPhaseMeasurand<ElectricCurrent>,
    ) -> Result<usize, rusqlite::Error> {
        let mut inserted_rows = 0;

        for phase in [Phase::L1, Phase::L2, Phase::L3] {
            match multi_phase_measurement.get_phase(phase) {
                Some(phase_measurand) => {
                    inserted_rows += db_connection.execute(
                        "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
                        (
                            name,
                            timestamp.timestamp_millis(),
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
        timestamp: DateTime<Utc>,
        meter_reading: &ElectricCurrent,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                timestamp.timestamp_millis(),
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
        timestamp: DateTime<Utc>,
        multi_phase_measurement: &MultiPhaseMeasurand<Energy>,
    ) -> Result<usize, rusqlite::Error> {
        let mut inserted_rows = 0;

        for phase in [Phase::L1, Phase::L2, Phase::L3] {
            match multi_phase_measurement.get_phase(phase) {
                Some(phase_measurand) => {
                    inserted_rows += db_connection.execute(
                        "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
                        (
                            name,
                            timestamp.timestamp_millis(),
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
        timestamp: DateTime<Utc>,
        meter_reading: &Energy,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                timestamp.timestamp_millis(),
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
        _timestamp: DateTime<Utc>,
        _multi_phase_measurement: &MultiPhaseMeasurand<Frequency>,
    ) -> Result<usize, rusqlite::Error> {
        Ok(0)
    }

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        timestamp: DateTime<Utc>,
        meter_reading: &Frequency,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                timestamp.timestamp_millis(),
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
        timestamp: DateTime<Utc>,
        multi_phase_measurement: &MultiPhaseMeasurand<Power>,
    ) -> Result<usize, rusqlite::Error> {
        let mut inserted_rows = 0;

        for phase in [Phase::L1, Phase::L2, Phase::L3] {
            match multi_phase_measurement.get_phase(phase) {
                Some(phase_measurand) => {
                    inserted_rows += db_connection.execute(
                        "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
                        (
                            name,
                            timestamp.timestamp_millis(),
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
        timestamp: DateTime<Utc>,
        meter_reading: &Power,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                timestamp.timestamp_millis(),
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
        _timestamp: DateTime<Utc>,
        _multi_phase_measurement: &MultiPhaseMeasurand<TemperatureInterval>,
    ) -> Result<usize, rusqlite::Error> {
        Ok(0)
    }

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        timestamp: DateTime<Utc>,
        meter_reading: &TemperatureInterval,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                timestamp.timestamp_millis(),
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
        timestamp: DateTime<Utc>,
        multi_phase_measurement: &MultiPhaseMeasurand<ElectricPotential>,
    ) -> Result<usize, rusqlite::Error> {
        let mut inserted_rows = 0;

        for phase in [Phase::L1, Phase::L2, Phase::L3] {
            match multi_phase_measurement.get_phase(phase.clone()) {
                Some(phase_measurand) => {
                    inserted_rows += db_connection.execute(
                        "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
                        (
                            name,
                            timestamp.timestamp_millis(),
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
        timestamp: DateTime<Utc>,
        meter_reading: &ElectricPotential,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (
                name,
                timestamp.timestamp_millis(),
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
        _timestamp: DateTime<Utc>,
        _multi_phase_measurement: &MultiPhaseMeasurand<f64>,
    ) -> Result<usize, rusqlite::Error> {
        Ok(0)
    }

    fn store_phase(
        db_connection: &Connection,
        name: &str,
        timestamp: DateTime<Utc>,
        meter_reading: &f64,
    ) -> Result<usize, rusqlite::Error> {
        return db_connection.execute(
            "INSERT INTO meter_readings (name, timestamp, value, unit, phase) VALUES(?1, ?2, ?3, ?4, ?5);",
            (name, timestamp.timestamp_millis(), meter_reading, String::default(), String::default()),
        );
    }
}
