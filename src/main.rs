use std::error::Error;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use clap::{Arg, App, crate_version};
use rodio::{Sink, Source};
use std::time::{Duration, Instant};
use std::thread;
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
    let cpu_hz = matches.value_of("cpu-hz").unwrap().parse::<u64>()?;
    let cpu_cycle = 1000000 / cpu_hz;

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

    // Set up some stuff for the sound
    let device = rodio::default_output_device().unwrap();
    let sink = Sink::new(&device);

    // Bunch of stuff to control timing of the CPU, display, and timers
    let mut time = Instant::now();
    let mut cpu_dt = Duration::new(0, 0);
    let mut display_dt = Duration::new(0, 0);
    let mut timer_dt = Duration::new(0, 0);
    let one_cpu_cycle = Duration::from_micros(cpu_cycle);
    let one_dis_cycle = Duration::from_micros(16667);
    let one_tim_cycle = Duration::from_micros(16667);

    // Ugly bad hack to force the winit event_loop to acutally run constantly
    // Without this (at least on linux/x11) the event_loop slows itself down when there isn't
    // constant events (like moving your mouse around wildly), which slows down all the timings.
    // If I read the docs right ControlFlow::Poll should act like how I want, but it doesn't
    let el_proxy = event_loop.create_proxy();
    thread::spawn(move || {
        loop {
            el_proxy.send_event(()).unwrap();
            thread::sleep(Duration::new(0, 100));
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        let now = Instant::now();
        let dt = now.duration_since(time);
        cpu_dt += dt;
        display_dt += dt;
        timer_dt += dt;
        time = now;

        if cpu_dt >= one_cpu_cycle {
            chip8.run_cycle();
            cpu_dt -= one_cpu_cycle;
        }

        if let Event::RedrawRequested(_) = event {
            if display_dt >= one_dis_cycle {
                for (i, pixel) in pixels.get_frame().chunks_exact_mut(4).enumerate() {
                    if chip8.screen[i] == 1 {
                        pixel.copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF])
                    } else {
                        pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xFF])
                    }
                }
                display_dt -= one_dis_cycle;
            }
            pixels.render().unwrap();
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
        if timer_dt > one_tim_cycle {
            chip8.dec_timers();
            if chip8.play_sound() {
                let sine = rodio::source::SineWave::new(440);
                sink.append(sine.take_duration(Duration::from_millis(50)));
            }
            timer_dt -= one_tim_cycle;
        }

        window.request_redraw();
    });
}

