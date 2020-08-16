use anyhow::Result;
use nalgebra::Point3;
use std::fs;
use the_hard_way::{Engine, DrawType, Vertex, Camera};
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
        .build(&event_loop)?;

    let mut engine = Engine::new(&window, APP_NAME)?;

    let vertex = fs::read("shaders/triangle.vert.spv")?;
    let fragment = fs::read("shaders/triangle.frag.spv")?;
    let material = engine.load_material(&vertex, &fragment, DrawType::Triangles)?;

    let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

    let vertices = [
        Vertex {
            pos: [-0.5, -0.5],
            color: [1.0, 0.0, 0.0],
        },
        Vertex {
            pos: [0.5, -0.5],
            color: [0.0, 1.0, 0.0],
        },
        Vertex {
            pos: [0.5, 0.5],
            color: [0.0, 0.0, 1.0],
        },
        Vertex {
            pos: [-0.5, 0.5],
            color: [1.0, 1.0, 1.0],
        },
    ];

    let mesh = engine.add_object(&vertices[..], &indices[..], material)?;

    let mut camera = Camera {
        eye: Point3::new(-1.0, -1.0, -1.0),
        at: Point3::origin(),
        fovy: 45.0f32.to_radians(),
        clip_near: 0.1,
        clip_far: 100.0,
    };

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
                .next_frame(&camera, time)
                .expect("Frame failed to render");
            camera.eye[0] = time.cos();
            camera.eye[2] = time.sin();
            /*
               let end_time = std::time::Instant::now();
               println!(
               "FPS: {}",
               1_000_000.0 / (end_time - current_time).as_micros() as f32
               );
               */
        }
        _ => (),
    })
}
