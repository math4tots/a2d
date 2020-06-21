//! Just run test main
extern crate a2d;
use a2d::Instance;
use a2d::SpriteBatch;
use a2d::SpriteSheet;
use a2d::Graphics2D;
use futures::executor::block_on;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub fn main() {
    // simple_logger::init_with_level(log::Level::Debug).unwrap();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize {
            width: 800,
            height: 800,
        })
        .build(&event_loop)
        .unwrap();

    let mut state = block_on(Graphics2D::from_winit_window(&window));
    let sheet = SpriteSheet::from_bytes(&mut state, include_bytes!("happy-tree.png"));
    let mut batch = SpriteBatch::new(sheet);
    batch.add(Instance::new(
        [0.0, 0.0, 0.75, 0.75],
        [0.25, 0.25, 0.75, 0.75],
        3.14 / 3.0,
    ));
    batch.add(Instance::new(
        [0.0, 0.0, 0.5, 0.5],
        [0.0, 0.0, 0.25, 0.25],
        0.0,
    ));
    batch.add(Instance::new(
        [0.75, 0.75, 1.0, 1.0],
        [0.5, 0.5, 1.0, 1.0],
        0.0,
    ));
    batch.add(Instance::new(
        [0.0, 0.75, 0.2, 1.0],
        [0.5, 0.5, 1.0, 1.0],
        0.0,
    ));

    let start = std::time::SystemTime::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(_) => {
            {
                let instance = batch.get_mut(0);
                let dur = start.elapsed().unwrap().as_secs_f32();
                instance.set_rotation((dur / 6.0).fract() * 2.0 * std::f32::consts::PI);
            }
            state.render(&[&batch]);
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(VirtualKeyCode::Escape),
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            },
            WindowEvent::Resized(physical_size) => {
                state.resize(*physical_size);
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                state.resize(**new_inner_size);
            }
            _ => {}
        },
        _ => {}
    })
}