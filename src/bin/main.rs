#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

extern crate alloc;

use esp_hal::gpio::{Input, InputConfig, Level, Output, Pull};
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, main};
use esp_println::println;

#[path = "../config.rs"]
mod config;
#[path = "../discovery.rs"]
mod discovery;
#[path = "../mqtt.rs"]
mod mqtt;
#[path = "../network_clock.rs"]
mod network_clock;
#[path = "../wifi.rs"]
mod wifi;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// Required by the esp-idf bootloader.
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);
    // COEX (WiFi + BLE simultaneously) needs extra RAM
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    println!("Initializing radio...");
    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");

    let (mut wifi_controller, wifi_interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    println!("Connecting to WiFi \"{}\"...", config::WIFI_SSID);
    let network_stack = wifi::connect(
        &mut wifi_controller,
        wifi_interfaces,
        config::WIFI_SSID,
        config::WIFI_PASSWORD,
    );
    println!("WiFi connected, IP acquired");

    // LED 1 (GPIO2): on when MQTT connected
    let mut led_mqtt = Output::new(peripherals.GPIO2, Level::Low, Default::default());
    // LED 2 (GPIO4): on when lights are on
    let mut led_lights = Output::new(peripherals.GPIO4, Level::Low, Default::default());
    // Button (GPIO5 → GND): press toggles bedroom/dining/living room lights via HA
    let button = Input::new(
        peripherals.GPIO5,
        InputConfig::default().with_pull(Pull::Up),
    );
    let mut btn_prev_low = false;
    let mut btn_last_press = esp_hal::time::Instant::now();

    println!("Starting MQTT..");
    mqtt::run(
        network_stack,
        |switch_id, on| {
            if switch_id == "lights_on" {
                led_lights.set_level(if on { Level::High } else { Level::Low });
            }
        },
        |connected| {
            led_mqtt.set_level(if connected { Level::High } else { Level::Low });
        },
        || {
            let is_low = button.is_low();
            let debounced = btn_last_press.elapsed() > esp_hal::time::Duration::from_millis(200);
            if is_low && !btn_prev_low && debounced {
                btn_prev_low = true;
                btn_last_press = esp_hal::time::Instant::now();
                true
            } else {
                if !is_low {
                    btn_prev_low = false;
                }
                false
            }
        },
    );
}
