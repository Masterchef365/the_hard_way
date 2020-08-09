use anyhow::Result;
use nalgebra::Matrix4;
use the_hard_way::{DrawType, Engine, Object};
use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const APP_NAME: &str = "Engine demo app";

fn main() -> Result<()> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title(APP_NAME)
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    let mut engine = Engine::new(&window, APP_NAME)?;

    /*
    let material = engine.load_material(
        "../shaders/triangle.vert.spv",
        "../shaders/triangle.frag.spv",
        DrawType::Triangles,
    )?;
    let mesh = engine.load_mesh(&[], &[], &[])?;
    let objects = [Object {
        material,
        mesh,
        transform: Matrix4::identity(),
    }];
    */
    let objects = [];

    let start_time = std::time::Instant::now();
    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            *control_flow = ControlFlow::Poll;
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            _ => (),
        },
        Event::MainEventsCleared => {
            let current_time = std::time::Instant::now();
            let time = (current_time - start_time).as_millis() as f32 / 1000.0;
            engine
                .next_frame(&objects, &Matrix4::identity(), time)
                .expect("Frame failed to render");
        }
        Event::LoopDestroyed => {
            println!("Exiting cleanly");
        }
        _ => (),
    })
}
