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

// USBのアロケータをstaticに配置して、プログラム実行中にメモリから消えないようにします。
use core::option::Option;
static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<hal::usb::UsbBus>> = None;

const USB_REPORT_DELAY: u32 = 10_000;
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

    defmt::println!("USB Device Initialized.");

    // 'a'キーを押すレポートと離すレポートを準備します。
    let press_report = usbd_hid::descriptor::KeyboardReport {
        modifier: 0,
        reserved: 0,
        leds: 0,
        keycodes: [0x04, 0, 0, 0, 0, 0], // 'a'
    };
    let release_report = usbd_hid::descriptor::KeyboardReport {
        modifier: 0,
        reserved: 0,
        leds: 0,
        keycodes: [0, 0, 0, 0, 0, 0],
    };

    loop {
        // 1. キー検知回数を減らすために待ちを入れます。
        usb_sleep(USB_REPORT_DELAY, &mut usb_dev, &mut hid);

        // 2. キー押下情報を渡します。
        let _ = hid.push_input(&press_report);

        // 3. キー検知回数を減らすために待ちを入れます。
        usb_sleep(USB_REPORT_DELAY, &mut usb_dev, &mut hid);

        // 4. キー離上情報を渡します。
        let _ = hid.push_input(&release_report);

        defmt::println!("Sent 'a'");
    }
}

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
