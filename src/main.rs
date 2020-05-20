#![feature(stdsimd)]
#![no_std]
#![no_main]

use core::arch::arm::__nop;
use core::fmt::Write;

mod port;
mod sim;
mod watchdog;
mod mcg;
mod osc;
mod uart;

#[no_mangle]
pub extern fn main() {
    let (wdog,sim,mcg,osc,pin) = unsafe {
        (watchdog::Watchdog::new(),
         sim::Sim::new(),
         mcg::Mcg::new(),
         osc::Osc::new(),
         port::Port::new(port::PortName::C).pin(5))
    };

    wdog.disable();
    // Enable the crystal oscillator with 10pf of capacitance
    osc.enable(10);
    // Turn on the Port C clock gate
    sim.enable_clock(sim::Clock::PortC);
    // Set our clocks:
    // core: 72Mhz
    // peripheral: 36MHz
    // flash: 24MHz
    sim.set_dividers(1, 2, 3);
    // We would also set the USB divider here if we wanted to use it.

    // Now we can start setting up the MCG for our needs.
    if let mcg::Clock::Fei(mut fei) = mcg.clock() {
        // Our 16MHz xtal is "very fast", and needs to be divided
        // by 512 to be in the acceptable FLL range.
        fei.enable_xtal(mcg::OscRange::VeryHigh);
        let fbe = fei.use_external(512);

        // PLL is 27/6 * xtal == 72MHz
        let pbe = fbe.enable_pll(27, 6);
        pbe.use_pll();
    } else {
        panic!("Somehow the clock wasn't in FEI mode");
    }

    let mut gpio = pin.make_gpio();

    gpio.output();
    gpio.high();

    sim.enable_clock(sim::Clock::PortB);
    sim.enable_clock(sim::Clock::Uart0);

    let uart = unsafe {
        let rx = port::Port::new(port::PortName::B).pin(16).make_rx();
        let tx = port::Port::new(port::PortName::B).pin(17).make_tx();
        uart::Uart::new(0, Some(rx), Some(tx), (468, 24))
    };

    writeln!(uart, "Hello, World").unwrap();

    loop {
        gpio.low();
        for _ in 0..100_000 {
            unsafe {
                __nop();
            }
        }
        gpio.high();
        for _ in 0..100_000 {
            unsafe {
                __nop();
            }
        }
    }
}

#[panic_handler]
fn teensy_panic(_pi: &core::panic::PanicInfo) -> ! {
    loop {};
}

extern {
    fn _stack_top();
}

#[link_section = ".vectors"]
#[no_mangle]
pub static _VECTORS: [unsafe extern fn(); 2] = [
    _stack_top,
    main,
];

#[link_section = ".flashconfig"]
#[no_mangle]
pub static _FLASHCONFIG: [u8; 16] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xDE, 0xF9, 0xFF, 0xFF
];
