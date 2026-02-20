use heapless::String;

use crate::config;

/// HA MQTT Discovery topic prefix.
const DISCOVERY_PREFIX: &str = "homeassistant";

/// Base topic used with the `~` abbreviation in payloads.
const BASE_TOPIC: &str = "wooden-box";

// ---- Topics ----------------------------------------------------------------

pub fn discovery_topic_sensor(object_id: &str) -> String<128> {
    let mut s: String<128> = String::new();
    let _ = s.push_str(DISCOVERY_PREFIX);
    let _ = s.push_str("/sensor/");
    let _ = s.push_str(config::HA_DEVICE_ID);
    let _ = s.push('/');
    let _ = s.push_str(object_id);
    let _ = s.push_str("/config");
    s
}

pub fn discovery_topic_switch(object_id: &str) -> String<128> {
    let mut s: String<128> = String::new();
    let _ = s.push_str(DISCOVERY_PREFIX);
    let _ = s.push_str("/switch/");
    let _ = s.push_str(config::HA_DEVICE_ID);
    let _ = s.push('/');
    let _ = s.push_str(object_id);
    let _ = s.push_str("/config");
    s
}

// ---- Payloads --------------------------------------------------------------
// Uses HA abbreviations to keep JSON small enough for embedded buffers.
// `~` expands to the value of "~" (BASE_TOPIC) inside stat_t / cmd_t.

/// Discovery payload for a temperature sensor entity.
pub fn temperature_payload() -> String<512> {
    let mut s: String<512> = String::new();
    let _ = s.push_str(r#"{"~":""#);
    let _ = s.push_str(BASE_TOPIC);
    let _ = s.push_str(r#"","name":"Temperature","uniq_id":""#);
    let _ = s.push_str(config::HA_DEVICE_ID);
    let _ = s.push_str(r#"_temp","stat_t":"~/sensor/temperature/state","unit_of_meas":"\u00b0C","dev_cla":"temperature","dev":{"ids":[""#);
    let _ = s.push_str(config::HA_DEVICE_ID);
    let _ = s.push_str(r#""],"name":""#);
    let _ = s.push_str(config::HA_DEVICE_NAME);
    let _ = s.push_str(r#""}}"#);
    s
}

/// Discovery payload for a humidity sensor entity.
pub fn humidity_payload() -> String<512> {
    let mut s: String<512> = String::new();
    let _ = s.push_str(r#"{"~":""#);
    let _ = s.push_str(BASE_TOPIC);
    let _ = s.push_str(r#"","name":"Humidity","uniq_id":""#);
    let _ = s.push_str(config::HA_DEVICE_ID);
    let _ = s.push_str(r#"_hum","stat_t":"~/sensor/humidity/state","unit_of_meas":"%","dev_cla":"humidity","dev":{"ids":[""#);
    let _ = s.push_str(config::HA_DEVICE_ID);
    let _ = s.push_str(r#""],"name":""#);
    let _ = s.push_str(config::HA_DEVICE_NAME);
    let _ = s.push_str(r#""}}"#);
    s
}

/// Discovery payload for a switch/relay entity.
/// `object_id` should match the one used in `discovery_topic_switch`.
pub fn switch_payload(object_id: &str, display_name: &str) -> String<512> {
    let mut s: String<512> = String::new();
    let _ = s.push_str(r#"{"~":""#);
    let _ = s.push_str(BASE_TOPIC);
    let _ = s.push_str(r#"","name":""#);
    let _ = s.push_str(display_name);
    let _ = s.push_str(r#"","uniq_id":""#);
    let _ = s.push_str(config::HA_DEVICE_ID);
    let _ = s.push('_');
    let _ = s.push_str(object_id);
    let _ = s.push_str(r#"","stat_t":"~/switch/"#);
    let _ = s.push_str(object_id);
    let _ = s.push_str(r#"/state","cmd_t":"~/switch/"#);
    let _ = s.push_str(object_id);
    let _ = s.push_str(r#"/command","dev":{"ids":[""#);
    let _ = s.push_str(config::HA_DEVICE_ID);
    let _ = s.push_str(r#""],"name":""#);
    let _ = s.push_str(config::HA_DEVICE_NAME);
    let _ = s.push_str(r#""}}"#);
    s
}
