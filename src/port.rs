use volatile::Volatile;
use bit_field::BitField;

#[derive(Clone,Copy)]
pub enum PortName {
    C,
    B
}

#[repr(C,packed)]
pub struct Port {
    pcr: [Volatile<u32>; 32],
    gpclr: Volatile<u32>,
    gpchr: Volatile<u32>,
    reserved_0: [u8; 24],
    isfr: Volatile<u32>,
}

pub struct Pin {
    port: *mut Port,
    pin: usize
}

pub struct Gpio {
    gpio: *mut GpioBitband,
    pin: usize
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

pub struct Tx(u8);
pub struct Rx(u8);

impl Port {
    pub unsafe fn new(name: PortName) -> &'static mut Port {
        &mut * match name {
            PortName::C => 0x4004B000 as *mut Port,
            PortName::B => 0x4004A000 as *mut Port
        }
    }

    pub unsafe fn set_pin_mode(&mut self, p: usize, mode: u32) {
        self.pcr[p].update(|pcr| {
            pcr.set_bits(8..11, mode);
        });
    }

    pub unsafe fn pin(&mut self, p: usize) -> Pin {
        Pin { port: self, pin: p }
    }

    pub fn name(&self) -> PortName {
        let addr = (self as *const Port) as u32;
        match addr {
            0x4004B000 => PortName::C,
            0x4004A000 => PortName::B,
            _ => unreachable!()
        }
    }
}

impl Pin {
    pub fn make_gpio(self) -> Gpio {
        unsafe {
            let port = &mut *self.port;
            port.set_pin_mode(self.pin, 1);
            Gpio::new(port.name(), self.pin)
        }
    }

    pub fn make_rx(self) -> Rx {
        unsafe {
            let port = &mut *self.port;
            match (port.name(), self.pin) {
                (PortName::B, 16) => {
                    port.set_pin_mode(self.pin, 3);
                    Rx(0)
                },
                _ => panic!("Invalid serial RX pin")
            }
        }
    }

    pub fn make_tx(self) -> Tx {
        unsafe {
            let port = &mut *self.port;
            match (port.name(), self.pin) {
                (PortName::B, 17) => {
                    port.set_pin_mode(self.pin, 3);
                    Tx(0)
                },
                _ => panic!("Invalid serial TX pin")
            }
        }
    }
}

impl Gpio {
    pub unsafe fn new(port: PortName, pin: usize) -> Gpio {
        let gpio = match port {
            PortName::C => 0x43FE1000 as *mut GpioBitband,
            PortName::B => 0x43FE0800 as *mut GpioBitband
        };

        Gpio { gpio, pin }
    }

    pub fn output(&mut self) {
        unsafe {
            (*self.gpio).pddr[self.pin].write(1);
        }
    }

    pub fn high(&mut self) {
        unsafe {
            (*self.gpio).psor[self.pin].write(1);
        }
    }

    pub fn low(&mut self) {
        unsafe {
            (*self.gpio).pcor[self.pin].write(1);
        }
    }
}

impl Rx {
    pub fn uart(&self) -> u8 {
        self.0
    }
}

impl Tx {
    pub fn uart(&self) -> u8 {
        self.0
    }
}
