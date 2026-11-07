use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UsageEvent {
    pub id: String,
    pub plugin_id: String,
    pub meter: String,
    pub units: u64,
    pub timestamp: String,
}
pub trait UsageMeter {
    fn record(&self, event: &UsageEvent) -> Result<(), String>;
}
pub struct Noop;
impl UsageMeter for Noop {
    fn record(&self, _: &UsageEvent) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_op_meter_accepts_usage_events_without_side_effects() {
        let event = UsageEvent {
            id: "evt-1".into(),
            plugin_id: "px.optimizer".into(),
            meter: "compile".into(),
            units: 1,
            timestamp: "2026-10-30T12:00:00Z".into(),
        };
        assert!(Noop.record(&event).is_ok());
        let schema = schemars::schema_for!(UsageEvent);
        assert!(serde_json::to_value(schema).unwrap().is_object());
    }
}
