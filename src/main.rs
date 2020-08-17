use anyhow::{Context, Result};
use nalgebra::{Matrix4, Point3, Vector3};
use openxr as xr;
use std::fs;
use std::io::Write;
use std::time::Duration;
use the_hard_way::{Camera, DrawType, Engine, Vertex};
use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const APP_NAME: &str = "Engine demo app";

fn main() -> Result<()> {
    let entry = xr::Entry::load()
        .context("couldn't find the OpenXR loader; try enabling the \"static\" feature")?;

    let mut enabled_extensions = xr::ExtensionSet::default();
    enabled_extensions.khr_vulkan_enable = true;
    let xr_instance = entry.create_instance(
        &xr::ApplicationInfo {
            application_name: APP_NAME,
            application_version: 0,
            engine_name: "Prototype engine",
            engine_version: 0,
        },
        &enabled_extensions,
        &[],
    )?;
    let instance_props = xr_instance.properties()?;
    println!(
        "loaded OpenXR runtime: {} {}",
        instance_props.runtime_name, instance_props.runtime_version
    );

    let system = xr_instance
        .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
        .unwrap();

    let (mut session, mut engine) = Engine::new(&xr_instance, system, APP_NAME)?;

    std::thread::sleep_ms(3000);

    todo!("It's all good!")
    /*
    let vertex = fs::read("shaders/triangle.vert.spv")?;
    let fragment = fs::read("shaders/triangle.frag.spv")?;
    let material = engine.load_material(&vertex, &fragment, DrawType::Triangles)?;

    let vertices = [
        Vertex {
            pos: [-1.0, -1.0, -1.0],
            color: [0.0, 1.0, 1.0],
        },
        Vertex {
            pos: [1.0, -1.0, -1.0],
            color: [1.0, 0.0, 1.0],
        },
        Vertex {
            pos: [1.0, 1.0, -1.0],
            color: [1.0, 1.0, 0.0],
        },
        Vertex {
            pos: [-1.0, 1.0, -1.0],
            color: [0.0, 1.0, 1.0],
        },
        Vertex {
            pos: [-1.0, -1.0, 1.0],
            color: [1.0, 0.0, 1.0],
        },
        Vertex {
            pos: [1.0, -1.0, 1.0],
            color: [1.0, 1.0, 0.0],
        },
        Vertex {
            pos: [1.0, 1.0, 1.0],
            color: [0.0, 1.0, 1.0],
        },
        Vertex {
            pos: [-1.0, 1.0, 1.0],
            color: [1.0, 0.0, 1.0],
        },
        ];

    let indices = [
        0, 1, 3, 3, 1, 2, 1, 5, 2, 2, 5, 6, 5, 4, 6, 6, 4, 7, 4, 0, 7, 7, 0, 3, 3, 2, 7, 7, 2, 6,
        4, 5, 0, 0, 5, 1,
    ];

    let mesh = engine.add_object(&vertices[..], &indices[..], material)?;
    let mesh2 = engine.add_object(&vertices[..], &indices[..], material)?;

    let mut camera = Camera {
        eye: Point3::new(-4.0, 4.0, -4.0),
        at: Point3::origin(),
        fovy: 45.0f32.to_radians(),
        clip_near: 0.1,
        clip_far: 100.0,
    };

    let target_frame_time = Duration::from_micros(1_000_000 / 60);
    let mut frame_count = 0;

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
            let frame_start_time = std::time::Instant::now();
            let time_var = (frame_start_time - start_time).as_millis() as f32 / 1000.0;

            engine
                .next_frame(&camera, time_var)
                .expect("Frame failed to render");
            let frame_end_time = std::time::Instant::now();
            frame_count += 1;

            let frame_duration = frame_end_time - frame_start_time;
            print!(
                "\x1b[1K\rFPS: Actual: {} Possible: {}",
                frame_count / (frame_end_time - start_time).as_secs().max(1),
                1_000_000 / frame_duration.as_micros(),
            );
            std::io::stdout().lock().flush().unwrap();

            let transform = Matrix4::from_euler_angles(0.0, time_var, 0.0);
            engine.set_transform(mesh, transform);

            let transform = Matrix4::new_translation(&Vector3::new(0.5, 0.5, 0.5));
            engine.set_transform(mesh2, transform);
            //camera.eye[0] = time.cos();
            //camera.eye[2] = time.sin();

            if frame_duration < target_frame_time {
                std::thread::sleep(target_frame_time - frame_duration);
            }
        }
        _ => (),
    })
    */
}
