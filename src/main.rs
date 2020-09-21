use std::error::Error;
use minifb::{Key, Window, WindowOptions};
use clap::{Arg, App, crate_version};
use chipurat8::chip8::{Chip8, WIDTH, HEIGHT};


fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("Chipurat8")
        .version(crate_version!())
        .author("David Phillips")
        .about("Little Chip8 emulator")
        .arg(Arg::new("rom")
            .short('r')
            .long("rom")
            .about("CHIP8 rom file to load")
            .takes_value(true)
            .required(true))
        .get_matches();

    let rom = matches.value_of("rom").unwrap();

    let mut buffer: Vec<u32> = vec![0; (WIDTH) * (HEIGHT)];

    let mut chip8 = Chip8::new();
    chip8.init(rom);

    let mut window = Window::new(
        "Chipurat8",
        WIDTH*10,
        HEIGHT*10,
        WindowOptions::default()
    )?;

    window.limit_update_rate(Some(std::time::Duration::from_micros(4150)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        chip8.run_cycle();

        if chip8.draw_flag {
            let mut j = 0;
            for i in buffer.iter_mut() {
                if chip8.screen[j] == 1 {
                    *i = 0xFFFFFF;
                } else {
                    *i = 0;
                }
                j += 1;
            }
            window.update_with_buffer(&buffer, WIDTH, HEIGHT)?;
            chip8.draw_flag = false;
        } else {
            window.update();
        }

        chip8.keys = [0; 16];
        window.get_keys().map(|keys| {
            for t in keys {
                match t {
                    Key::Key1 => chip8.keys[1] = 1,
                    Key::Key2 => chip8.keys[2] = 1,
                    Key::Key3 => chip8.keys[3] = 1,
                    Key::Key4 => chip8.keys[0xC] = 1,
                    Key::Q => chip8.keys[4] = 1,
                    Key::W => chip8.keys[5] = 1,
                    Key::E => chip8.keys[6] = 1,
                    Key::R => chip8.keys[0xD] = 1,
                    Key::A => chip8.keys[7] = 1,
                    Key::S => chip8.keys[8] = 1,
                    Key::D => chip8.keys[9] = 1,
                    Key::F => chip8.keys[0xE] = 1,
                    Key::Z => chip8.keys[0xA] = 1,
                    Key::X => chip8.keys[0] = 1,
                    Key::C => chip8.keys[0xB] = 1,
                    Key::V => chip8.keys[0xF] = 1,
                    _ => ()
                }
            }
        });
    }

    Ok(())
}
