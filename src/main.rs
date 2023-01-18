#![no_std]
#![no_main]

use core::cell::RefCell;
use critical_section::Mutex;
use esp32c3_hal::{
    clock::ClockControl,
    gpio::Gpio9,
    gpio::{Event, Input, Pin, PullUp},
    interrupt,
    pac::{self, Peripherals},
    prelude::*,
    timer::TimerGroup,
    Rtc, IO,
};
use core::sync::atomic::{AtomicUsize, Ordering};
use esp_backtrace as _;

static BUTTON: Mutex<RefCell<Option<Gpio9<Input<PullUp>>>>> = Mutex::new(RefCell::new(None));
static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[riscv_rt::entry]
fn main() -> ! {
    let peripherals = Peripherals::take().unwrap();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();


    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut button = io.pins.gpio9.into_pull_up_input();
    button.listen(Event::FallingEdge);

    critical_section::with(|cs| BUTTON.borrow_ref_mut(cs).replace(button));

    interrupt::enable(pac::Interrupt::GPIO, interrupt::Priority::Priority3).unwrap();

    loop {
        COUNTER.fetch_add(1, Ordering::SeqCst);
    }
}

#[interrupt]
fn GPIO() {
    critical_section::with(|cs| {
        esp_println::println!("GPIO interrupt, counter = {:}", COUNTER.load(Ordering::SeqCst));
        COUNTER.store(0, Ordering::SeqCst);
        BUTTON
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .clear_interrupt();
    });
}