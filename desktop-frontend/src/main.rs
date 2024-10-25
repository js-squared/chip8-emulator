use std::env;
use std::fs::File;
use std::io::Read;
use backend::*;
use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::keyboard::Keycode;
use sdl2::audio::{AudioCallback, AudioSpecDesired};

const SCALE: u32 = 15;
const WINDOW_WIDTH: u32 = (SCREEN_WIDTH as u32) * SCALE;
const WINDOW_HEIGHT: u32 = (SCREEN_HEIGHT as u32) * SCALE;
const TICKS_PER_FRAME: usize = 10;

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("usage: cargo run path/to/game");
        return;
    }

    // Setup SDL
    let sdl_context = sdl2::init().unwrap();

    // Setup video
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("Chip-8 Emulator",
                                        WINDOW_WIDTH,
                                        WINDOW_HEIGHT)
                                .position_centered()
                                .opengl()
                                .build()
                                .unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.clear();
    canvas.present();

    // Setup audio
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(1), // mono
        samples: None,     // default sample size
    };

    let device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        // initialize the audio callback
        SquareWave {
            phase_inc: 440.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.25,
        }
    }).unwrap();

    let mut previous_sound = false;
    let mut sound: bool;

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut chip8 = Processor::new();

    let mut rom = File::open(&args[1]).expect("Unable to open file");
    let mut buffer = Vec::new();

    rom.read_to_end(&mut buffer).unwrap();
    chip8.load(&buffer);

    'gameloop: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit{..} => {
                    break 'gameloop;
                },
                Event::KeyDown{keycode: Some(key), ..} => {
                    if let Some(k) = key_to_button(key) {
                        chip8.keypress(k, true);
                    }
                },
                Event::KeyUp{keycode: Some(key), ..} => {
                    if let Some(k) = key_to_button(key) {
                        chip8.keypress(k, false);
                    }
                },
                _ => ()
            }
        }

        for _ in 0..TICKS_PER_FRAME {
            chip8.tick();
        }
        chip8.tick_timers();
        draw_screen(&chip8, &mut canvas);
        sound = chip8.get_sound();
        if sound && !previous_sound {
            device.resume();
        } else if !sound && previous_sound {
            device.pause();
        }
        previous_sound = sound;
    }
}

fn draw_screen(processor: &Processor, canvas: &mut Canvas<Window>) {
    // Clear canvas as black
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    let screen_buffer = processor.get_display();
    // Now set draw color to white, iterate through each point and
    // see if it should be drawn
    canvas.set_draw_color(Color::RGB(255, 255, 255));
    for (i, pixel) in screen_buffer.iter().enumerate() {
        if *pixel {
            // Convert our 1D array's index into a 2D (x,y) position
            let x = (i % SCREEN_WIDTH) as u32;
            let y = (i / SCREEN_WIDTH) as u32;
            // Draw a rectangle at (x,y), scaled up by our SCALE value
            let rect = Rect::new((x * SCALE) as i32,
                                 (y * SCALE) as i32,
                                 SCALE,
                                 SCALE);
            canvas.fill_rect(rect).unwrap();
        }
    }
    canvas.present();
}

fn key_to_button(key: Keycode) -> Option<usize> {
    match key {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    }
}
