use std::time::{Duration, SystemTime, UNIX_EPOCH};

use context_attribute::context;
use framework::MainOutput;
use types::cycle_time::CycleTime;

#[derive(Deserialize, Serialize)]
pub struct CycleTimeWarnings {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl CycleTimeWarnings {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs {})
    }
}
