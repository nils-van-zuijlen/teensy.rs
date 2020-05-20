use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::sim::ClockGate;
use volatile::Volatile;
use bit_field::BitField;

#[derive(Clone,Copy)]
pub enum PortName {
    C,
    B
}

#[repr(C,packed)]
struct PortRegs {
    pcr: [Volatile<u32>; 32],
    gpclr: Volatile<u32>,
    gpchr: Volatile<u32>,
    reserved_0: [u8; 24],
    isfr: Volatile<u32>,
}

pub struct Port {
    reg: UnsafeCell<&'static mut PortRegs>,
    locks: [AtomicBool; 32],
    _gate: ClockGate,
}

pub struct Pin<'a> {
    port: &'a Port,
    pin: usize
}

pub struct Gpio<'a> {
    gpio: *mut GpioBitband,
    pin: Pin<'a>
}

#[repr(C,packed)]
struct GpioBitband {
    pdor: [Volatile<u32>; 32],
    psor: [Volatile<u32>; 32],
    pcor: [Volatile<u32>; 32],
    ptor: [Volatile<u32>; 32],
    pdir: [Volatile<u32>; 32],
    pddr: [Volatile<u32>; 32]
}

pub struct Tx<'a> {
    uart: u8,
    _pin: Pin<'a>
}
pub struct Rx<'a> {
    uart: u8,
    _pin: Pin<'a>
}

impl Port {
    pub unsafe fn new(name: PortName, gate: ClockGate) -> Port {
        let myself = &mut * match name {
            PortName::C => 0x4004B000 as *mut PortRegs,
            PortName::B => 0x4004A000 as *mut PortRegs
        };

        Port { reg: UnsafeCell::new(myself), locks: Default::default(), _gate: gate}
    }

    pub unsafe fn set_pin_mode(&self, p: usize, mode: u32) {
        assert!(p < 32);
        self.reg().pcr[p].update(|pcr| {
            pcr.set_bits(8..11, mode);
        });
    }

    pub fn pin(&self, p: usize) -> Pin {
        assert!(p < 32);
        let was_init = self.locks[p].swap(true, Ordering::Relaxed);
        if was_init {
            panic!("Pin {} is already in use", p);
        }
        Pin { port: self, pin: p }
    }

    unsafe fn drop_pin(&self, p: usize) {
        assert!(p < 32);
        self.locks[p].store(false, Ordering::Relaxed);
    }

    pub fn name(&self) -> PortName {
        let addr = (self as *const Port) as u32;
        match addr {
            0x4004B000 => PortName::C,
            0x4004A000 => PortName::B,
            _ => unreachable!()
        }
    }

    fn reg(&self) -> &'static mut PortRegs {
        // NOTE: This does no validation. It's on the calling
        // functions to ensure they're not accessing the same
        // registers from multiple codepaths. If they can't make those
        // guarantees, they should be marked as `unsafe` (See
        // `set_pin_mode` as an example).
        unsafe {
            *self.reg.get()
        }
    }
}

impl<'a> Pin<'a> {
    pub fn make_gpio(self) -> Gpio<'a> {
        unsafe {
            self.port.set_pin_mode(self.pin, 1);
            Gpio::new(self.port.name(), self)
        }
    }

    pub fn make_rx(self) -> Rx<'a> {
        unsafe {
            match (self.port.name(), self.pin) {
                (PortName::B, 16) => {
                    self.port.set_pin_mode(self.pin, 3);
                    Rx{uart: 0, _pin: self}
                },
                _ => panic!("Invalid serial RX pin")
            }
        }
    }

    pub fn make_tx(self) -> Tx<'a> {
        unsafe {
            match (self.port.name(), self.pin) {
                (PortName::B, 17) => {
                    self.port.set_pin_mode(self.pin, 3);
                    Tx{uart: 0, _pin: self}
                },
                _ => panic!("Invalid serial TX pin")
            }
        }
    }
}

impl <'a> Drop for Pin<'a> {
    fn drop(&mut self) {
        unsafe {
            self.port.drop_pin(self.pin);
        }
    }
}

impl<'a> Gpio<'a> {
    pub unsafe fn new(port: PortName, pin: Pin) -> Gpio {
        let gpio = match port {
            PortName::C => 0x43FE1000 as *mut GpioBitband,
            PortName::B => 0x43FE0800 as *mut GpioBitband
        };

        Gpio { gpio, pin }
    }

    pub fn output(&mut self) {
        unsafe {
            (*self.gpio).pddr[self.pin.pin].write(1);
        }
    }

    pub fn high(&mut self) {
        unsafe {
            (*self.gpio).psor[self.pin.pin].write(1);
        }
    }

    pub fn low(&mut self) {
        unsafe {
            (*self.gpio).pcor[self.pin.pin].write(1);
        }
    }
}

impl Rx<'_> {
    pub fn uart(&self) -> u8 {
        self.uart
    }
}

impl Tx<'_> {
    pub fn uart(&self) -> u8 {
        self.uart
    }
}
