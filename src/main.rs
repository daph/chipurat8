use std::error::Error;
use pixels::{PixelsBuilder, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use clap::{Arg, App, crate_version};
use rodio::{Sink, Source};
use std::time::{Duration, Instant};
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
        .arg(Arg::new("cpu-hz")
            .short('c')
            .long("cpu-hz")
            .about("HZ to set the CPU to operate at")
            .takes_value(true)
            .default_value("500"))
        .get_matches();

    let rom = matches.value_of("rom").unwrap();
    let cpu_hz = matches.value_of("cpu-hz").unwrap().parse::<f64>()?;

    let cycles_per_frame = (cpu_hz / 60.0) as u64;
    let extra_cycle_every = (((cpu_hz / 60.0) % 1.0) * 10.0) as u64;

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
        PixelsBuilder::new(WIDTH as u32, HEIGHT as u32, surface_texture)
            .enable_vsync(true)
            .build()?
    };

    // Set up some stuff for the sound
    let device = rodio::default_output_device().unwrap();
    let sink = Sink::new(&device);

    // Control the timing of our updates
    let mut time = Instant::now();
    let mut update_dt = Duration::new(0, 0);
    let update_rate = Duration::from_micros(16667);

    let mut frame_count = 0;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        let now = Instant::now();
        let dt = now.duration_since(time);
        update_dt += dt;
        time = now;

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

        match event {
            Event::MainEventsCleared => {
                if update_dt >= update_rate {

                    for _ in 0..cycles_per_frame {
                        chip8.run_cycle();
                    }

                    // run an extra cycle every few frames to catch up to our target hz
                    // if our target hz is not evenly divisble by 60
                    if frame_count >= extra_cycle_every && extra_cycle_every != 0 {
                        chip8.run_cycle();
                        frame_count = 0;
                    }

                    for (i, pixel) in pixels.get_frame().chunks_exact_mut(4).enumerate() {
                        if chip8.screen[i] == 1 {
                            pixel.copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF])
                        } else {
                            pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xFF])
                        }
                    }

                    if extra_cycle_every > 0 {
                        frame_count += 1;
                    }

                    chip8.dec_timers();
                    if chip8.play_sound() {
                        let sine = rodio::source::SineWave::new(440);
                        sink.append(sine.take_duration(Duration::from_millis(50)));
                    }

                    update_dt -= update_rate;
                }
                pixels.render().unwrap();
            },
            _ => (),
        };

        window.request_redraw();
    });
}

