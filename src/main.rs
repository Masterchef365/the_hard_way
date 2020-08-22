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

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use rand::Rng;
use std::time::Instant;

const APP_NAME: &str = "Engine demo app";

fn main() -> Result<()> {
    // Handle interrupts gracefully
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    })
    .expect("setting Ctrl-C handler");
	
	let path = std::path::Path::new(r"C:\Users\Duncan\Documents\openxrs\sys\OpenXR-SDK\build_dynamic\src\loader\Release\openxr_loader.dll");
    println!("OPENXR PATH: {:?} {}", path, path.exists());
	let entry = xr::Entry::load_from(path)
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

    let floor_cube = engine.add_object(&vertices[..], &indices[..], material)?;
    let position = Vector3::new(0.0, -1.0, 0.0);
    let translation = Matrix4::new_translation(&position);
    engine.set_transform(floor_cube, translation);

    let mut rng = rand::thread_rng();
    let mut cubes = Vec::new();
    for _ in 0..100 {
        let cube = engine.add_object(&vertices[..], &indices[..], material)?;
        let position = Vector3::new(
            rng.gen_range(-30.0, 30.0),
            rng.gen_range(0.0, 30.0),
            rng.gen_range(-30.0, 30.0),
        );
        let translation = Matrix4::new_translation(&position);
        engine.set_transform(cube, translation);
        cubes.push((cube, translation));
    }

    let mut event_storage = xr::EventDataBuffer::new();
    let mut session_running = false;

    let start_time = std::time::Instant::now();
    'main_loop: loop {
        if !running.load(Ordering::Relaxed) {
            println!("requesting exit");
            // The OpenXR runtime may want to perform a smooth transition between scenes, so we
            // can't necessarily exit instantly. Instead, we must notify the runtime of our
            // intent and wait for it to tell us when we're actually done.
            match session.request_exit() {
                Ok(()) => {}
                Err(xr::sys::Result::ERROR_SESSION_NOT_RUNNING) => break,
                Err(e) => panic!("{}", e),
            }
        }

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

        //let start = Instant::now();
        engine.next_frame(&xr_instance, &session, system, 0.0)?;
        //let end = Instant::now();
        //println!("{}", end.duration_since(start).as_millis());

        let frame_start_time = std::time::Instant::now();
        let time_var = (frame_start_time - start_time).as_micros() as f32 / 1_000_000.0;

        /*
        let rotation = Matrix4::from_euler_angles(0.0, time_var, 0.0);
        for (cube, trans) in &cubes {
            engine.set_transform(*cube, trans * rotation);
        }
        */
    }

    drop(session);

    Ok(())
}
