use crate::FroniusApi;

#[derive(Default)]
pub struct FroniusMock {
    pub unblock_battery_called: bool,
    pub block_battery_for_duration_called: bool
}

impl FroniusApi for FroniusMock {
    fn fully_unblock_battery(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.unblock_battery_called = true;
        Ok(())
    }

    fn block_battery_for_duration(
            &mut self,
            _duration_to_block: &std::time::Duration,
        ) -> Result<(), Box<dyn std::error::Error>> {
        self.block_battery_for_duration_called = true;
        Ok(())
    }
}
