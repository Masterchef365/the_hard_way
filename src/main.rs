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

    let mut event_storage = xr::EventDataBuffer::new();
    let mut session_running = false;

    'main_loop: loop {
        while let Some(event) = xr_instance.poll_event(&mut event_storage).unwrap() {
            use xr::Event::*;
            match event {
                SessionStateChanged(e) => {
                    println!("entered state {:?}", e.state());
                    match e.state() {
                        xr::SessionState::READY => {
                            session
                                .begin(xr::ViewConfigurationType::PRIMARY_STEREO)
                                .unwrap();
                            session_running = true;
                        }
                        xr::SessionState::STOPPING => {
                            session.end().unwrap();
                            session_running = false;
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            println!("EXITING");
                            break 'main_loop;
                        }
                        _ => {}
                    }
                }
                InstanceLossPending(_) => {
                    println!("Pending instance loss");
                    break 'main_loop;
                }
                EventsLost(e) => {
                    println!("lost {} events", e.lost_event_count());
                }
                _ => {}
            }
        }

        if !session_running {
            // Don't grind up the CPU
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }

        engine.next_frame(&xr_instance, &session, system, &camera, 0.0)?;

        /*
        let transform = Matrix4::from_euler_angles(0.0, time_var, 0.0);
        engine.set_transform(mesh, transform);

        let transform = Matrix4::new_translation(&Vector3::new(0.5, 0.5, 0.5));
        engine.set_transform(mesh2, transform);
        */
    }

    drop(session);

    Ok(())
}
