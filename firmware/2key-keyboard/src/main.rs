#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_probe as _;
use rp_pico as bsp;

use bsp::hal;
use hal::pac;
use usb_device::prelude::*;

use usbd_hid::descriptor::KeyboardReport;
use usbd_hid::descriptor::SerializedDescriptor;
use embedded_hal::digital::v2::{InputPin, OutputPin};

// USBのアロケータをstaticに配置して、プログラム実行中にメモリから消えないようにします。
use core::option::Option;
static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<hal::usb::UsbBus>> = None;

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

const USB_REPORT_DELAY: u32 = 1_000;
fn usb_sleep(
    count: u32,
    usb_dev: &mut usb_device::device::UsbDevice<'static, hal::usb::UsbBus>,
    hid: &mut usbd_hid::hid_class::HIDClass<'static, hal::usb::UsbBus>,
) {
    for _ in 0..count {
        // UsbBus をポーリングし、HIDクラスにディスパッチします。
        // USB に準拠するには、USB ホストに接続している間、
        // 少なくとも 10 ミリ秒ごとに 1 回呼び出す必要があるため、このようにループで実行します。
        usb_dev.poll(&mut [hid]);
        cortex_m::asm::nop();
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    // ペリフェラルの取得、これをHALやusb_deviceクレートに渡すことで制御を実現します。
    let mut p = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(p.WATCHDOG);

    // クロックとpllsを初期化します。
    let clocks = hal::clocks::init_clocks_and_plls(
        // マイコンはクリスタルの周波数を知らないため、BSPからの情報を渡します。
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

    // USBの通信チャネルを提供してくれるBus allocatorを準備します。
    let usb_bus = hal::usb::UsbBus::new(
        p.USBCTRL_REGS,
        p.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut p.RESETS,
    );

    // アロケータをstaticな領域に固定します。
    unsafe {
        USB_BUS = Some(usb_device::bus::UsbBusAllocator::new(usb_bus));
    }
    let bus_allocator = unsafe { USB_BUS.as_ref().unwrap() };

    // ベンダーIDとプロダクトIDを設定します。今回は任意の値にします。
    let vid_pid = usb_device::device::UsbVidPid(0x6666, 0x0487);

    // HIDクラスを準備します。
    let mut hid = usbd_hid::hid_class::HIDClass::new(bus_allocator, KeyboardReport::desc(), 60);

    // USBデバイスを作成します。
    let mut usb_dev = UsbDeviceBuilder::new(bus_allocator, vid_pid)
        .manufacturer("yu2ta7ka")
        .product("RustyKeysImitation")
        .serial_number("487")
        .build();

    // Single Cycle Input and Output (SIO)ブロックを分割利用できるようにします。
    let sio = hal::Sio::new(p.SIO);
    // Pins構造体を介して、ピンのシングルトンインスタンスを取得します。   
    let pins = bsp::Pins::new(p.IO_BANK0, p.PADS_BANK0, sio.gpio_bank0, &mut p.RESETS);
    // 2行(row)1列(col)のキーマトリクスとしてピンを利用します。  
    let mut col1 = pins.gpio16.into_push_pull_output();
    let row1 = pins.gpio22.into_pull_down_input();
    let row2 = pins.gpio21.into_pull_down_input();

    defmt::println!("USB Device Initialized.");

    let mut last_report = [0u8; 6];

    // 初期化後少し長めに待ちます。
    usb_sleep(100_000, &mut usb_dev, &mut hid);

    loop {

        let mut keys = [0u8; 6];
        let mut num_keys = 0;

        // キーマトリクス回路からキー押下検知します。
        col1.set_high().ok().unwrap();
        usb_sleep(USB_REPORT_DELAY, &mut usb_dev, &mut hid);
        if row1.is_high().ok().unwrap() {
            // 'y'キー押下情報を格納します。 
            keys[num_keys] = 0x1f; // '2'
            num_keys += 1;
            defmt::println!("key 22");
        }
        if row2.is_high().ok().unwrap() {
            // '2'キー押下情報を格納します。 
            keys[num_keys] = 0x1c; // 'y'
            defmt::println!("key 21");
        }
        col1.set_low().ok().unwrap();

        // 状態が変化した時だけ送信します。
        if keys != last_report {
            //キーボードのレポートディスクリプタを準備します。
            let report = KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: keys,
            };

            if hid.push_input(&report).is_ok() {
                last_report = keys;
                // チャタリング防止とPC側の受信待ち
                usb_sleep(USB_REPORT_DELAY, &mut usb_dev, &mut hid);
            }
        }
    }
}
