#![no_main]
#![no_std]

use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use panic_probe as _;

use bsp::hal;
use hal::pac;
use rp_pico as bsp;

use usb_device as usbd;
use usbd::prelude::UsbDeviceBuilder;
use usbd_hid::descriptor::SerializedDescriptor;

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

fn sleep() {
    for _ in 0..10000 {
        cortex_m::asm::nop();
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, world! yu2ta7ka");

    let mut p = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(p.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        p.XOSC,
        p.CLOCKS,
        p.PLL_SYS,
        p.PLL_USB,
        &mut p.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let bus = hal::usb::UsbBus::new(
        p.USBCTRL_REGS,
        p.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut p.RESETS,
    );
    let usb_bus_allocator = usbd::class_prelude::UsbBusAllocator::new(bus);

    let vid_pid = usbd::device::UsbVidPid(0x6666, 0x0487);
    let mut hid = usbd_hid::hid_class::HIDClass::new(
        &usb_bus_allocator,
        usbd_hid::descriptor::KeyboardReport::desc(),
        60,
    );
    let mut dev = UsbDeviceBuilder::new(&usb_bus_allocator, vid_pid)
        .manufacturer("yu2ta7ka")
        .product("RustyKeysImitation")
        .serial_number("487")
        .build();

    let sio = hal::Sio::new(p.SIO);
    let pins = bsp::Pins::new(p.IO_BANK0, p.PADS_BANK0, sio.gpio_bank0, &mut p.RESETS);
    let mut col1 = pins.gpio16.into_push_pull_output();
    let row1 = pins.gpio22.into_pull_down_input();
    let row2 = pins.gpio21.into_pull_down_input();

    loop {
        let mut keys: [u8; 6] = [0u8; 6];
        let mut num_keys: usize = 0;
        dev.poll(&mut [&mut hid]);

        col1.set_high().ok().unwrap();
        sleep();
        if row1.is_high().ok().unwrap() {
            keys[num_keys] = 0x1f;
            num_keys += 1;
            defmt::println!("key 22");
        }
        if row2.is_high().ok().unwrap() {
            keys[num_keys] = 0x1c;
            num_keys += 1;
            defmt::println!("key 21");
        }
        sleep();
        col1.set_low().ok().unwrap();

        let report = usbd_hid::descriptor::KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: keys,
        };
        hid.push_input(&report).ok();
    }

    exit()
}