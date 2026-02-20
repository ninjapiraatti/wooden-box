use embedded_time::{clock::Error, fraction::Fraction, Clock, Instant};

/// Clock adapter that satisfies `embedded_time::Clock` using esp-hal's monotonic timer.
///
/// smoltcp-nal requires this trait to manage TCP timeouts and retransmissions.
/// 1 tick = 1 millisecond (SCALING_FACTOR = 1/1000 second per tick).
pub struct EspClock;

impl Clock for EspClock {
    type T = u32;
    // 1 tick = 1 ms = 1/1000 s
    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        // esp-hal Instant is a newtype — access time via duration_since_epoch().
        // 1 MHz internal timer: as_millis() gives u64 milliseconds since boot.
        // u32 ms overflows at ~49 days uptime, which is acceptable.
        let ms = esp_hal::time::Instant::now()
            .duration_since_epoch()
            .as_millis() as u32;
        Ok(Instant::new(ms))
    }
}
