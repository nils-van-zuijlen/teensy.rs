use crate::port::Rx;
use crate::port::Tx;
use crate::uart::Uart;
use crate::port::PortName;
use crate::port::Port;
use volatile::Volatile;
use bit_field::BitField;

use core::sync::atomic::{AtomicBool,Ordering};

#[repr(C,packed)]
struct SimRegs {
    sopt1: Volatile<u32>,
    sopt1_cfg: Volatile<u32>,
    _pad0: [u32; 1023],
    sopt2: Volatile<u32>,
    _pad1: Volatile<u32>,
    sopt4: Volatile<u32>,
    sopt5: Volatile<u32>,
    _pad2: Volatile<u32>,
    sopt7: Volatile<u32>,
    _pad3: [u32; 2],
    sdid: Volatile<u32>,
    _pad4: [u32; 3],
    scgc4: Volatile<u32>,
    scgc5: Volatile<u32>,
    scgc6: Volatile<u32>,
    scgc7: Volatile<u32>,
    clkdiv1: Volatile<u32>,
    clkviv2: Volatile<u32>,
    fcfg1: Volatile<u32>,
    fcfg2: Volatile<u32>,
    uidh: Volatile<u32>,
    uidmh: Volatile<u32>,
    uidml: Volatile<u32>,
    uidl: Volatile<u32>
}

pub struct Sim {
    reg: &'static mut SimRegs
}

pub struct ClockGate {
    gate: &'static mut Volatile<u32>
}

impl ClockGate {
    fn new(reg: usize, bit: usize) -> ClockGate {
        assert!(reg <= 7);
        assert!(bit <= 31);
        let base: usize = 0x42900500;
        let reg_offset = 128 * (reg - 1);
        let bit_offset = 4 * bit;
        let ptr = (base + reg_offset + bit_offset) as *mut Volatile<u32>;
        unsafe {
            ClockGate { gate: &mut *ptr }
        }
    }
}

impl Drop for ClockGate {
    fn drop(&mut self) {
        self.gate.write(0);
    }
}

static SIM_INIT: AtomicBool = AtomicBool::new(false);

impl Sim {
    pub fn new() -> Sim {
        let was_init = SIM_INIT.swap(true, Ordering::SeqCst);
        if was_init {
            panic!("Cannot initialize SIM: It's already active");
        }
        let reg = unsafe {&mut *(0x40047000 as *mut SimRegs)};
        Sim {reg}
    }

    pub fn set_dividers(&mut self, core: u32, bus: u32, flash: u32) {
        let mut clkdiv: u32 = 0;
        clkdiv.set_bits(28..32, core-1);
        clkdiv.set_bits(24..28, bus-1);
        clkdiv.set_bits(16..20, flash-1);
        unsafe {
            self.reg.clkdiv1.write(clkdiv);
        }
    }

    pub fn port(&mut self, port: PortName) -> Port {
        let gate = match port {
            PortName::B => ClockGate::new(5, 10),
            PortName::C => ClockGate::new(5, 11),
        };
        if gate.gate.read() != 0 {
            panic!("Cannot create Port instance; it is already in use");
        }
        gate.gate.write(1);
        unsafe {
            Port::new(port, gate)
        }
    }

    pub fn uart<'a, 'b>(&mut self, uart: u8, rx: Option<Rx<'a>>, tx: Option<Tx<'b>>, clkdiv: (u16, u8)) -> Uart<'a, 'b> {
        let gate = match uart {
            0 => ClockGate::new(4, 10),
            _ => panic!("Cannot enable clock for UART {}", uart)
        };
        if gate.gate.read() != 0 {
            panic!("Cannot create Uart instance; it is already in use");
        }
        gate.gate.write(1);
        unsafe {
            Uart::new(uart, rx, tx, clkdiv, gate)
        }
    }
}

impl Drop for Sim {
    fn drop(&mut self) {
        SIM_INIT.store(false, Ordering::SeqCst);
    }
}
