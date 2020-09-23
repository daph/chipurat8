use std::error::Error;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use clap::{Arg, App, crate_version};
use chipurat8::chip8::{Chip8, WIDTH, HEIGHT};

const KEY_MAP: [(VirtualKeyCode, usize); 16] = [
    (VirtualKeyCode::Key1, 0x1),
    (VirtualKeyCode::Key2, 0x2),
    (VirtualKeyCode::Key3, 0x3),
    (VirtualKeyCode::Key4, 0xC),
    (VirtualKeyCode::Q,    0x4),
    (VirtualKeyCode::W,    0x5),
    (VirtualKeyCode::E,    0x6),
    (VirtualKeyCode::R,    0xD),
    (VirtualKeyCode::A,    0x7),
    (VirtualKeyCode::S,    0x8),
    (VirtualKeyCode::D,    0x9),
    (VirtualKeyCode::F,    0xE),
    (VirtualKeyCode::Z,    0xA),
    (VirtualKeyCode::X,    0x0),
    (VirtualKeyCode::C,    0xB),
    (VirtualKeyCode::V,    0xF),
];


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

    let mut chip8 = Chip8::new();
    chip8.init(rom);

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new((WIDTH*4) as f64, (HEIGHT*4) as f64);
        WindowBuilder::new()
            .with_title("Chipurat8")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)?
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture)?
    };

    event_loop.run(move |event, _, control_flow| {
        chip8.run_cycle();

        if let Event::RedrawRequested(_) = event {
            for (i, pixel) in pixels.get_frame().chunks_exact_mut(4).enumerate() {
                if chip8.screen[i] == 1 {
                    pixel.copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF])
                } else {
                    pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xFF])
                }
            }

            pixels.render().unwrap()
        }

        if input.update(&event) {
            for (k, n) in KEY_MAP.iter() {
                if input.key_pressed(*k) {
                    chip8.keys[*n] = 1
                }
                if input.key_released(*k) {
                    chip8.keys[*n] = 0
                }
            }

            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if let Some(size) = input.window_resized() {
                pixels.resize(size.width, size.height);
            }
        }

        window.request_redraw()
    });
}

