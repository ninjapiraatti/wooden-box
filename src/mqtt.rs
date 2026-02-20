use core::net::{IpAddr, Ipv4Addr};

use embedded_nal::TcpClientStack;
use embedded_time::Clock;
use heapless::String;
use minimq::{
    broker::{Broker, IpBroker},
    mqtt_client::MqttClient,
    types::TopicFilter,
    ConfigBuilder, Minimq, Publication, QoS, Will,
};

use esp_println::println;

use crate::{config, discovery, network_clock::EspClock};

const BASE: &str = "wooden-box";

/// Enter the MQTT poll loop — never returns.
///
/// Call this after WiFi + DHCP are established.
/// `on_command(switch_id, on)` is called whenever HA sends a switch command.
/// `on_connect_change(connected)` is called whenever the MQTT connection state changes.
/// `poll_button()` is called every loop iteration; return `true` once per debounced press.
pub fn run<S, F, G, H>(network: S, mut on_command: F, mut on_connect_change: G, mut poll_button: H) -> !
where
    S: TcpClientStack,
    F: FnMut(&str, bool),
    G: FnMut(bool),
    H: FnMut() -> bool,
{
    let broker = IpBroker::new(IpAddr::V4(Ipv4Addr::new(
        config::MQTT_BROKER_IP[0],
        config::MQTT_BROKER_IP[1],
        config::MQTT_BROKER_IP[2],
        config::MQTT_BROKER_IP[3],
    )));

    let mut mqtt_buf = [0u8; 2048];

    let mut will_topic: String<64> = String::new();
    let _ = will_topic.push_str(BASE);
    let _ = will_topic.push_str("/status");

    let mut mqtt: Minimq<'_, S, EspClock, IpBroker> = Minimq::new(
        network,
        EspClock,
        ConfigBuilder::new(broker, &mut mqtt_buf)
            .client_id(config::MQTT_CLIENT_ID)
            .expect("client ID too long")
            .set_auth(config::MQTT_USERNAME, config::MQTT_PASSWORD)
            .expect("auth config failed")
            .keepalive_interval(30)
            .will(Will::new(will_topic.as_str(), b"offline", &[]).expect("will failed"))
            .expect("will config failed"),
    );

    let mut subscribed = false;
    let mut last_sensor_publish = esp_hal::time::Instant::now();
    let sensor_interval = esp_hal::time::Duration::from_secs(30);

    let mut poll_errors: u32 = 0;
    let mut loop_counter: u32 = 0;
    let mut last_status_print = esp_hal::time::Instant::now();
    let mut was_connected = false;

    let ip = config::MQTT_BROKER_IP;
    println!(
        "MQTT loop starting, broker {}.{}.{}.{}:{}",
        ip[0], ip[1], ip[2], ip[3], config::MQTT_BROKER_PORT
    );

    loop {
        loop_counter += 1;

        // Print connection status every 5 seconds
        if last_status_print.elapsed() >= esp_hal::time::Duration::from_secs(5) {
            println!(
                "MQTT status: loop={}, connected={}, errors={}",
                loop_counter,
                mqtt.client().is_connected(),
                poll_errors
            );
            last_status_print = esp_hal::time::Instant::now();
        }
        // 1. Drive the MQTT state machine; dispatch incoming messages
        if let Err(e) = mqtt.poll(|client, topic, payload, _props| {
            let prefix = {
                let mut p: String<64> = String::new();
                let _ = p.push_str(BASE);
                let _ = p.push_str("/switch/");
                p
            };

            if let Some(rest) = topic.strip_prefix(prefix.as_str()) {
                if let Some(switch_id) = rest.strip_suffix("/command") {
                    let on = payload == b"ON";
                    on_command(switch_id, on);

                    // Echo state back to HA
                    let mut state_topic: String<64> = String::new();
                    let _ = state_topic.push_str(BASE);
                    let _ = state_topic.push_str("/switch/");
                    let _ = state_topic.push_str(switch_id);
                    let _ = state_topic.push_str("/state");

                    let _ = client.publish(
                        Publication::new(
                            state_topic.as_str(),
                            if on { b"ON".as_ref() } else { b"OFF".as_ref() },
                        )
                        .retain()
                        .qos(QoS::AtMostOnce),
                    );
                }
            }

            // Lights automation state → LED 2
            let lights_topic = {
                let mut t: String<64> = String::new();
                let _ = t.push_str(BASE);
                let _ = t.push_str("/automation/lights_on");
                t
            };
            if topic == lights_topic.as_str() {
                let on = payload == b"ON";
                on_command("lights_on", on);
            }

            None::<()>
        }) {
            poll_errors += 1;
            // Log every error, but throttle to avoid flooding
            if poll_errors <= 5 || poll_errors % 100 == 0 {
                println!("MQTT poll error #{}: {:?}", poll_errors, e);
            }
        }

        // 2. Notify on connection state change (drives LED 1)
        {
            let now_connected = mqtt.client().is_connected();
            if now_connected != was_connected {
                was_connected = now_connected;
                on_connect_change(now_connected);
            }
        }

        // 3. Publish button toggle when pressed
        if poll_button() {
            let mut topic: String<64> = String::new();
            let _ = topic.push_str(BASE);
            let _ = topic.push_str("/button/toggle");
            let _ = mqtt.client().publish(
                Publication::new(topic.as_str(), b"pressed").qos(QoS::AtMostOnce),
            );
        }

        // 4. Subscribe and publish discovery once connected; reset on disconnect
        {
            let client = mqtt.client();

            if client.is_connected() {
                if !subscribed {
                    println!("MQTT connected, publishing discovery...");
                    let mut cmd_filter: String<64> = String::new();
                    let _ = cmd_filter.push_str(BASE);
                    let _ = cmd_filter.push_str("/switch/+/command");

                    let mut lights_filter: String<64> = String::new();
                    let _ = lights_filter.push_str(BASE);
                    let _ = lights_filter.push_str("/automation/lights_on");

                    let _ = client.subscribe(
                        &[
                            TopicFilter::new(cmd_filter.as_str()),
                            TopicFilter::new(lights_filter.as_str()),
                        ],
                        &[],
                    );

                    publish_discovery(client);

                    let mut status_topic: String<64> = String::new();
                    let _ = status_topic.push_str(BASE);
                    let _ = status_topic.push_str("/status");

                    let _ = client.publish(
                        Publication::new(status_topic.as_str(), b"online")
                            .retain()
                            .qos(QoS::AtMostOnce),
                    );

                    subscribed = true;
                }

                // 5. Publish sensor readings on a timer
                if last_sensor_publish.elapsed() >= sensor_interval {
                    // TODO: replace placeholder values with actual sensor reads
                    publish_sensor(client, "temperature", 0.0);
                    publish_sensor(client, "humidity", 0.0);
                    last_sensor_publish = esp_hal::time::Instant::now();
                }
            } else {
                subscribed = false;
            }
        }

        // Small yield to avoid spinning at 100% CPU
        let t = esp_hal::time::Instant::now();
        while t.elapsed() < esp_hal::time::Duration::from_millis(10) {}
    }
}

fn publish_discovery<S, Clk, B>(client: &mut MqttClient<'_, S, Clk, B>)
where
    S: TcpClientStack,
    Clk: Clock,
    B: Broker,
{
    // Each heapless::String must be kept alive while publish() uses its slice.
    let t = discovery::discovery_topic_sensor("temperature");
    let p = discovery::temperature_payload();
    let _ = client.publish(Publication::new(t.as_str(), p.as_bytes()).retain().qos(QoS::AtMostOnce));

    let t = discovery::discovery_topic_sensor("humidity");
    let p = discovery::humidity_payload();
    let _ = client.publish(Publication::new(t.as_str(), p.as_bytes()).retain().qos(QoS::AtMostOnce));

    let t = discovery::discovery_topic_switch("relay1");
    let p = discovery::switch_payload("relay1", "Relay 1");
    let _ = client.publish(Publication::new(t.as_str(), p.as_bytes()).retain().qos(QoS::AtMostOnce));
}

pub fn publish_sensor<S, Clk, B>(client: &mut MqttClient<'_, S, Clk, B>, sensor: &str, value: f32)
where
    S: TcpClientStack,
    Clk: Clock,
    B: Broker,
{
    let mut topic: String<64> = String::new();
    let _ = topic.push_str(BASE);
    let _ = topic.push_str("/sensor/");
    let _ = topic.push_str(sensor);
    let _ = topic.push_str("/state");

    let formatted = format_one_decimal(value);
    let _ = client.publish(
        Publication::new(topic.as_str(), formatted.as_bytes()).qos(QoS::AtMostOnce),
    );
}

/// Format a float as "XX.X" (one decimal) without std float formatting.
fn format_one_decimal(value: f32) -> String<16> {
    let mut s: String<16> = String::new();
    let is_neg = value < 0.0;
    let abs = if is_neg { -value } else { value };
    let integer = abs as u32;
    let frac = ((abs - integer as f32) * 10.0) as u32;

    if is_neg {
        let _ = s.push('-');
    }
    if integer >= 100 {
        let _ = s.push(char::from_digit(integer / 100, 10).unwrap_or('0'));
    }
    if integer >= 10 {
        let _ = s.push(char::from_digit((integer / 10) % 10, 10).unwrap_or('0'));
    }
    let _ = s.push(char::from_digit(integer % 10, 10).unwrap_or('0'));
    let _ = s.push('.');
    let _ = s.push(char::from_digit(frac, 10).unwrap_or('0'));
    s
}
