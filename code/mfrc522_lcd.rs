#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate embedded_hal;
extern crate hd44780_driver;
extern crate mfrc522;
extern crate numtoa;
extern crate stellaris_launchpad;
extern crate tm4c123x_hal;

use core::alloc::Layout;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v1_compat::OldOutputPin;
use embedded_hal::digital::v2::OutputPin;
use hd44780_driver::{Cursor, CursorBlink, Display, DisplayMode, HD44780};
use mfrc522::Mfrc522;
use numtoa::NumToA;
use stellaris_launchpad::board;
use tm4c123x_hal::gpio::GpioExt;

const MASTER_CARD: [u8; 4] = [192, 33, 232, 239];

#[no_mangle]
pub fn stellaris_main(mut board: stellaris_launchpad::board::Board) {
    let mut delay = tm4c123x_hal::delay::Delay::new(
        board.core_peripherals.SYST,
        stellaris_launchpad::board::clocks(),
    );

    let pins_a = board.GPIO_PORTA.split(&board.power_control);
    let pins_c = board.GPIO_PORTC.split(&board.power_control);
    let pins_d = board.GPIO_PORTD.split(&board.power_control);

    let rs = pins_a.pa2.into_push_pull_output();
    let en = pins_d.pd6.into_push_pull_output();
    let b4 = pins_c.pc7.into_push_pull_output();
    let b5 = pins_c.pc6.into_push_pull_output();
    let b6 = pins_c.pc5.into_push_pull_output();
    let b7 = pins_c.pc4.into_push_pull_output();

    let mut pins_b = board.GPIO_PORTB.split(&board.power_control);
    let sck = pins_b.pb4.into_af_push_pull(&mut pins_b.control);
    let miso = pins_b.pb6.into_af_push_pull(&mut pins_b.control);
    let mosi = pins_b.pb7.into_af_push_pull(&mut pins_b.control);
    let nss = pins_b.pb5.into_push_pull_output();

    let spi = tm4c123x_hal::spi::Spi::spi2(
        board.SSI2,
        (sck, miso, mosi),
        mfrc522::MODE,
        tm4c123x_hal::time::Hertz(1_000_000),
        board::clocks(),
        &board.power_control,
    );

    let mut mfrc522 = Mfrc522::new(spi, OldOutputPin::from(nss)).unwrap();

    let mut lcd = HD44780::new_4bit(rs, en, b4, b5, b6, b7, &mut delay).unwrap();
    lcd.reset(&mut delay).unwrap();
    lcd.clear(&mut delay).unwrap();

    lcd.set_display_mode(
        DisplayMode {
            display: Display::On,
            cursor_visibility: Cursor::Invisible,
            cursor_blink: CursorBlink::Off,
        },
        &mut delay,
    )
    .unwrap();

    let mut buffer = [0u8; 10];
    lcd.write_str("Access Control", &mut delay).unwrap();
    lcd.set_cursor_pos(40, &mut delay).unwrap();
    lcd.write_str("<<Scan Your Card", &mut delay).unwrap();

    loop {
        if let Ok(atqa) = mfrc522.reqa() {
            if let Ok(uid) = mfrc522.select(&atqa) {
                lcd.clear(&mut delay).unwrap();
                lcd.set_cursor_pos(0, &mut delay).unwrap();
                lcd.write_str("ID: ", &mut delay).unwrap();

                let card_uid = uid.bytes();
                for byte in card_uid {
                    lcd.write_str(byte.numtoa_str(16, &mut buffer), &mut delay)
                        .unwrap();
                }
                if card_uid == &MASTER_CARD {
                    board.led_green.set_high().unwrap();
                    lcd.set_cursor_pos(40, &mut delay).unwrap();
                    lcd.write_str("Access Granted!", &mut delay).unwrap();
                    delay.delay_ms(500u32);
                    board.led_green.set_low().unwrap();
                } else {
                    board.led_red.set_high().unwrap();
                    lcd.set_cursor_pos(40, &mut delay).unwrap();
                    lcd.write_str("Access Denied!", &mut delay).unwrap();
                    delay.delay_ms(500u32);
                    board.led_red.set_low().unwrap();
                }
                lcd.clear(&mut delay).unwrap();
                lcd.write_str("Access Control", &mut delay).unwrap();
                lcd.set_cursor_pos(40, &mut delay).unwrap();
                lcd.write_str("<<Scan Your Card", &mut delay).unwrap();
            }
        }
    }
}

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    board::panic();
}
