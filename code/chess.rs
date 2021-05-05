#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate keypad;

extern crate alloc;
extern crate arrayvec;
extern crate chess_engine;
extern crate embedded_hal;
extern crate hd44780_driver;
extern crate numtoa;
extern crate stellaris_launchpad;
extern crate tm4c123x_hal;

use alloc::string::ToString;
use arrayvec::ArrayString;
use chess_engine::*;
use core::alloc::Layout;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use hd44780_driver::{Cursor, CursorBlink, Display, DisplayMode, HD44780};
use numtoa::NumToA;
use stellaris_launchpad::board;
use tm4c123x_hal::gpio::GpioExt;

use tm4c123x_hal::gpio::{
    gpioa::{PA2, PA5, PA6, PA7},
    gpiob::{PB0, PB1, PB4},
    gpioc::{PC4, PC5, PC6, PC7},
    gpiod::PD6,
    gpioe::{PE4, PE5},
};
use tm4c123x_hal::gpio::{Input, Output, PullUp, PushPull};

const BUFFER_SIZE: usize = 10;

keypad_struct! {
    struct MyKeypad {
        rows: (
            PE5<Input<PullUp>>,
            PE4<Input<PullUp>>,
            PB1<Input<PullUp>>,
            PB0<Input<PullUp>>,
        ),
        columns: (
            PB4<Output<PushPull>>,
            PA5<Output<PushPull>>,
            PA6<Output<PushPull>>,
            PA7<Output<PushPull>>,
        ),
    }
}

#[no_mangle]
pub fn stellaris_main(mut board: stellaris_launchpad::board::Board) {
    let mut delay = tm4c123x_hal::delay::Delay::new(
        board.core_peripherals.SYST,
        stellaris_launchpad::board::clocks(),
    );

    let pins_a = board.GPIO_PORTA.split(&board.power_control);
    let pins_c = board.GPIO_PORTC.split(&board.power_control);
    let pins_d = board.GPIO_PORTD.split(&board.power_control);
    let pins_b = board.GPIO_PORTB.split(&board.power_control);
    let pins_e = board.GPIO_PORTE.split(&board.power_control);

    let rs = pins_a.pa2.into_push_pull_output();
    let en = pins_d.pd6.into_push_pull_output();
    let b4 = pins_c.pc7.into_push_pull_output();
    let b5 = pins_c.pc6.into_push_pull_output();
    let b6 = pins_c.pc5.into_push_pull_output();
    let b7 = pins_c.pc4.into_push_pull_output();

    let r1 = pins_e.pe5.into_pull_up_input();
    let r2 = pins_e.pe4.into_pull_up_input();
    let r3 = pins_b.pb1.into_pull_up_input();
    let r4 = pins_b.pb0.into_pull_up_input();

    let c1 = pins_b.pb4.into_push_pull_output();
    let c2 = pins_a.pa5.into_push_pull_output();
    let c3 = pins_a.pa6.into_push_pull_output();
    let c4 = pins_a.pa7.into_push_pull_output();

    let keypad = keypad_new!(MyKeypad {
        rows: (r1, r2, r3, r4),
        columns: (c1, c2, c3, c4),
    });

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

    //let keys = keypad.decompose();
    //let first_key = &keys[0][0];
    //if first_key.is_low().unwrap() {
    //    lcd.write_str("ifkl 1", &mut delay).unwrap();
    //} else {
    //    lcd.write_str("ifkl 0", &mut delay).unwrap();
    //}
    //delay.delay_ms(1000u32);

    let mut chess_board = BoardBuilder::default()
        .piece(Piece::Pawn(WHITE, A2))
        .piece(Piece::Pawn(WHITE, B2))
        .piece(Piece::Pawn(WHITE, C2))
        .piece(Piece::Pawn(WHITE, F2))
        .piece(Piece::Pawn(WHITE, G2))
        .piece(Piece::Pawn(WHITE, H2))
        .piece(Piece::Pawn(BLACK, B6))
        .piece(Piece::Pawn(BLACK, A7))
        .piece(Piece::Pawn(BLACK, C7))
        .piece(Piece::Pawn(BLACK, F7))
        .piece(Piece::Pawn(BLACK, G7))
        .piece(Piece::Pawn(BLACK, H7))
        .piece(Piece::Knight(WHITE, D5))
        .piece(Piece::Knight(BLACK, G8))
        .piece(Piece::Bishop(WHITE, E3))
        .piece(Piece::Bishop(BLACK, D6))
        .piece(Piece::Rook(WHITE, A1))
        .piece(Piece::Rook(WHITE, E1))
        .piece(Piece::Rook(BLACK, D8))
        .piece(Piece::Rook(BLACK, H8))
        .piece(Piece::Queen(WHITE, C4))
        .piece(Piece::Queen(BLACK, G6))
        .piece(Piece::King(WHITE, G1))
        .piece(Piece::King(BLACK, C8))
        .build();

    let mut buffer = [0u8; BUFFER_SIZE];
    let mut is_player_turn: bool = true;
    lcd.set_cursor_pos(40, &mut delay).unwrap();
    lcd.write_str("Player's turn!", &mut delay).unwrap();

    loop {
        let chess_move: Move = if is_player_turn {
            player_turn(&keypad, &mut lcd, &mut delay)
        } else {
            lcd.set_cursor_pos(40, &mut delay).unwrap();
            lcd.write_str("                    ", &mut delay).unwrap();
            lcd.set_cursor_pos(40, &mut delay).unwrap();
            lcd.write_str("Evaluating...", &mut delay).unwrap();
            board.led_blue.set_high().unwrap();
            let (cpu_move, count, _) = chess_board.get_best_next_move(2); // SLOW!
            board.led_blue.set_low().unwrap();

            lcd.set_cursor_pos(40, &mut delay).unwrap();
            lcd.write_str("                    ", &mut delay).unwrap();
            lcd.set_cursor_pos(40, &mut delay).unwrap();
            lcd.write_str("CPU: ", &mut delay).unwrap();

            match cpu_move {
                Move::Piece(from_pos, to_pos) => {
                    lcd.write_str(conv_file(from_pos.get_col()), &mut delay)
                        .unwrap();
                    lcd.write_str(conv_rank(from_pos.get_row()), &mut delay)
                        .unwrap();
                    lcd.write_str(conv_file(to_pos.get_col()), &mut delay)
                        .unwrap();
                    lcd.write_str(conv_rank(to_pos.get_row()), &mut delay)
                        .unwrap();
                }
                Move::KingSideCastle => lcd.write_str("O-O", &mut delay).unwrap(),
                Move::QueenSideCastle => lcd.write_str("O-O-O", &mut delay).unwrap(),
                Move::Resign => lcd.write_str("resigns", &mut delay).unwrap(),
            }

            lcd.write_char(' ', &mut delay).unwrap();
            lcd.write_str(count.numtoa_str(10, &mut buffer), &mut delay)
                .unwrap();
            cpu_move
        };

        match chess_board.play_move(chess_move) {
            GameResult::IllegalMove(_e) => {
                lcd.set_cursor_pos(40, &mut delay).unwrap();
                lcd.write_str("                    ", &mut delay).unwrap();
                lcd.set_cursor_pos(40, &mut delay).unwrap();
                // it may panic here if not handle correctly
                lcd.write_str("Illegal move!", &mut delay).unwrap();
                continue;
            }
            GameResult::Victory(color) => {
                lcd.clear(&mut delay).unwrap();
                let winner: &str = match color {
                    Color::White => "White",
                    Color::Black => "Black",
                };
                lcd.write_str(winner, &mut delay).unwrap();
                lcd.write_str(" wins.", &mut delay).unwrap();
                break;
            }
            GameResult::Stalemate => {
                lcd.clear(&mut delay).unwrap();
                lcd.write_str("Stalemated", &mut delay).unwrap();
                break;
            }
            GameResult::Continuing(next_board) => {
                chess_board = next_board;
                is_player_turn = !is_player_turn;
            }
        }
    }

    loop {
        board.led_green.set_high().unwrap();
        delay.delay_ms(500u32);
        board.led_green.set_low().unwrap();
        board.led_blue.set_high().unwrap();
        delay.delay_ms(500u32);
        board.led_blue.set_low().unwrap();
        delay.delay_ms(500u32);
    }
}

fn player_turn<'a>(
    keypad: &MyKeypad,
    lcd: &mut HD44780<
        hd44780_driver::bus::FourBitBus<
            PA2<Output<PushPull>>,
            PD6<Output<PushPull>>,
            PC7<Output<PushPull>>,
            PC6<Output<PushPull>>,
            PC5<Output<PushPull>>,
            PC4<Output<PushPull>>,
        >,
    >,
    delay: &mut tm4c123x_hal::delay::Delay,
) -> Move {
    lcd.set_cursor_pos(0, delay).unwrap();
    lcd.write_str("                    ", delay).unwrap();
    lcd.set_cursor_pos(0, delay).unwrap();
    lcd.write_str("Player: ", delay).unwrap();

    let from_file = get_chess_file(keypad);
    let from_file_str = conv_file(from_file);
    lcd.write_str(from_file_str, delay).unwrap();
    let from_rank = get_chess_rank(keypad);
    let from_rank_str = conv_rank(from_rank);
    lcd.write_str(from_rank_str, delay).unwrap();

    let to_file = get_chess_file(keypad);
    let to_file_str = conv_file(to_file);
    lcd.write_str(to_file_str, delay).unwrap();
    let to_rank = get_chess_rank(keypad);
    let to_rank_str = conv_rank(to_rank);
    lcd.write_str(to_rank_str, delay).unwrap();

    match (from_file, from_rank, to_file, to_rank) {
        (0, 0, 0, 0) => {
            lcd.set_cursor_pos(0, delay).unwrap();
            lcd.write_str("                    ", delay).unwrap();
            lcd.set_cursor_pos(0, delay).unwrap();
            lcd.write_str("Player: O-O", delay).unwrap();
        }
        (1, 1, 1, 1) => {
            lcd.set_cursor_pos(0, delay).unwrap();
            lcd.write_str("                    ", delay).unwrap();
            lcd.set_cursor_pos(0, delay).unwrap();
            lcd.write_str("Player: O-O-O", delay).unwrap();
        }
        _ => {}
    }

    let notation = get_notation(
        from_file,
        from_file_str,
        from_rank,
        from_rank_str,
        to_file,
        to_file_str,
        to_rank,
        to_rank_str,
    );

    // attempt to parse the notation, may panic
    let player_move: Move = Move::parse(notation.to_string()).unwrap();
    player_move
}

fn get_notation(
    from_file: i32,
    from_file_str: &str,
    from_rank: i32,
    from_rank_str: &str,
    to_file: i32,
    to_file_str: &str,
    to_rank: i32,
    to_rank_str: &str,
) -> ArrayString<BUFFER_SIZE> {
    let mut ret_string = ArrayString::<BUFFER_SIZE>::new();
    match (from_file, from_rank, to_file, to_rank) {
        (0, 0, 0, 0) => ret_string.push_str("O-O"),
        (1, 1, 1, 1) => ret_string.push_str("O-O-O"),
        _ => {
            ret_string.push_str(from_file_str);
            ret_string.push_str(from_rank_str);
            ret_string.push_str(to_file_str);
            ret_string.push_str(to_rank_str);
        }
    }
    ret_string
}

fn conv_file<'a>(file: i32) -> &'a str {
    return match file {
        0 => "a",
        1 => "b",
        2 => "c",
        3 => "d",
        4 => "e",
        5 => "f",
        6 => "g",
        7 => "h",
        _ => "u",
    };
}

fn conv_rank<'a>(file: i32) -> &'a str {
    return match file {
        0 => "1",
        1 => "2",
        2 => "3",
        3 => "4",
        4 => "5",
        5 => "6",
        6 => "7",
        7 => "8",
        _ => "u",
    };
}

fn get_chess_file(keypad: &MyKeypad) -> i32 {
    // row column - file - ret
    // 33 a 0; 32 b 1; 31 c 2; 30 d 3
    // 23 e 4; 22 f 5; 21 g 6; 20 h 7
    let keys = keypad.decompose();
    loop {
        for (row_index, row) in keys.iter().enumerate() {
            for (col_index, key) in row.iter().enumerate() {
                if key.is_low().unwrap() {
                    match (row_index, col_index) {
                        (3, 3) => return 0,
                        (3, 2) => return 1,
                        (3, 1) => return 2,
                        (3, 0) => return 3,
                        (2, 3) => return 4,
                        (2, 2) => return 5,
                        (2, 1) => return 6,
                        (2, 0) => return 7,
                        (_, _) => continue,
                    };
                }
            }
        }
    }
}

fn get_chess_rank(keypad: &MyKeypad) -> i32 {
    // row column - rank - ret
    // 13 1 0; 12 2 1; 11 3 2; 10 4 3
    // 03 5 4; 02 6 5; 01 7 6; 00 8 7
    let keys = keypad.decompose();
    loop {
        for (row_index, row) in keys.iter().enumerate() {
            for (col_index, key) in row.iter().enumerate() {
                if key.is_low().unwrap() {
                    match (row_index, col_index) {
                        (1, 3) => return 0,
                        (1, 2) => return 1,
                        (1, 1) => return 2,
                        (1, 0) => return 3,
                        (0, 3) => return 4,
                        (0, 2) => return 5,
                        (0, 1) => return 6,
                        (0, 0) => return 7,
                        (_, _) => continue,
                    };
                }
            }
        }
    }
}

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    board::panic();
}
