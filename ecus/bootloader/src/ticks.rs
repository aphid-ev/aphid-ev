use crate::pac::SYST;
use boot_protocol::Timeout;
use cortex_m::peripheral::syst::SystClkSource;

/// SysTick reloads every 144 000 cycles: one millisecond at the 144 MHz
/// system clock `init_clock` sets up.
const RELOAD: u32 = 144_000 - 1;

/// A free-running millisecond tick from SysTick, for [`Timeout`]s.
///
/// The tick is SysTick's COUNTFLAG, which clears when read. Only one loop
/// polls it at a time; a send inside the block loop takes the ticks that
/// pass during it, which only makes the block timeout a little longer.
pub struct Ticks {
    syst: SYST,
}

impl Ticks {
    pub fn new(mut syst: SYST) -> Self {
        syst.set_clock_source(SystClkSource::Core);
        syst.set_reload(RELOAD);
        syst.clear_current();
        syst.enable_counter();

        Self { syst }
    }

    /// Advances `timeout` if a millisecond has passed since the last call,
    /// and returns whether it has expired.
    pub fn poll(&mut self, timeout: &mut Timeout) -> bool {
        if self.syst.has_wrapped() {
            timeout.tick()
        } else {
            timeout.expired()
        }
    }

    /// Puts SysTick back in its reset state. It is a core peripheral with
    /// no reset bit in RCC.
    pub fn reset(mut self) {
        self.syst.disable_counter();
        self.syst.disable_interrupt();
        self.syst.set_reload(0);
        self.syst.clear_current();
        self.syst.set_clock_source(SystClkSource::External);
    }
}
