
extern crate fluxcore;
extern crate nalgebra;
extern crate sdl2;
extern crate clap;

use std::time::Duration;
use std::str::FromStr;

use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::image::{INIT_PNG, INIT_JPG};

use nalgebra::{Point3, Vector3};

use fluxcore::manager::*;
use fluxcore::constants::DEFAULT_PORT;
use fluxcore::job::JobConfiguration;
use fluxcore::scene::*;
use fluxcore::shapes::*;
use fluxcore::color::Color;

use clap::{App, Arg};

const DEFAULT_SAMPLE_ROOT: usize = 1;
const DEFAULT_DEPTH: usize = 5;

fn main() {
    let config = config_from_args();

    println!("{:?}", config);

    let c = JobConfiguration {
        rows_per_work_unit: config.rows_per_work_unit,
        max_trace_depth: config.max_depth,
        sample_root: config.sample_root,
    };

    let s = SceneData {
        scene_name: String::from("test_scene"),
        output_settings: OutputSettings {
            image_width: 800,
            image_height: 600,
            pixel_size: 0.5,
        },
        camera_settings: CameraSettings::new(
                             Point3::new(2.5, 1.5, -9.0),
                             Point3::new(2.5, 1.0, 0.0),
                             Vector3::new(0.0, 1.0, 0.0)),
        camera_data: CameraData {
            zoom_factor: 1.0,
            view_plane_distance: 500.0,
            focal_distance: 10.0,
            lens_radius: 0.0,
        },
        background: Color::all(0.0),
        shapes: vec![
            // Environment light
            ShapeData::Sphere(SphereData {
                center: Point3::new(0.0, 0.0, 0.0),
                radius: 100.0,
                invert: true,
                material: MaterialData::Emissive(EmissiveData {
                    color: Color::new(1.0, 0.9686, 0.8588),
                    power: 1.0,
                })
            }),
            ShapeData::Sphere(SphereData {
                center: Point3::new(0.0, 1.0, 0.0),
                radius: 1.0,
                invert: false,
                material: MaterialData::Matte(MatteData {
                    diffuse_coefficient: 1.0,
                    ambient_color: Color::white(),
                    diffuse_color: Color::new(0.0, 0.7, 0.6),
                })
            }),
            ShapeData::Sphere(SphereData {
                center: Point3::new(2.0, 1.0, 2.0),
                radius: 1.0,
                invert: false,
                material: MaterialData::GlossyReflective(GlossyReflectiveData {
                    reflect_amount: 0.9,
                    reflect_color: Color::new(0.9, 1.0, 0.9),
                    reflect_exponent: 100.0,
                })
            }),
            ShapeData::Sphere(SphereData {
                center: Point3::new(4.0, 1.0, 4.0),
                radius: 1.0,
                invert: false,
                material: MaterialData::GlossyReflective(GlossyReflectiveData {
                    reflect_amount: 0.9,
                    reflect_color: Color::new(0.9, 1.0, 0.9),
                    reflect_exponent: 100_000.0,
                })
            }),
            ShapeData::Sphere(SphereData {
                center: Point3::new(6.0, 1.0, 2.0),
                radius: 1.0,
                invert: false,
                material: MaterialData::Matte(MatteData {
                    diffuse_coefficient: 1.0,
                    ambient_color: Color::white(),
                    diffuse_color: Color::new(0.5, 0.3, 0.8),
                })
            }),
            ShapeData::Plane(PlaneData {
                point: Point3::new(0.0, 0.0, 0.0),
                normal: Vector3::new(0.0, 1.0, 0.0),
                material: MaterialData::Matte(MatteData {
                    diffuse_coefficient: 1.0,
                    ambient_color: Color::white(),
                    diffuse_color: Color::all(0.5),
                })
            }),
        ],
    };

    // Set up workers ////////////////////////////////////////////////////////

    if !config.use_local_worker && config.network_workers.is_empty() {
        println!("No workers specified, exiting");
        return;
    }

    let mut worker_handles: Vec<WorkerHandle> = vec![];
    let mut local_worker: Option<LocalWorker> = None;
    let mut net_workers: Vec<NetworkWorker> = vec![];

    if config.use_local_worker {
        let worker = LocalWorker::new();
        worker_handles.push(worker.handle());
        local_worker = Some(worker);
    }

    for endpoint in config.network_workers {
        println!("Connecting to {}", &endpoint);
        let worker = NetworkWorker::new(&endpoint);
        worker_handles.push(worker.handle());
        net_workers.push(worker);
    }

    // SDL setup /////////////////////////////////////////////////////////////
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let _image_context = sdl2::image::init(INIT_PNG | INIT_JPG).unwrap();

    let image_width = s.output_settings.image_width;
    let image_height = s.output_settings.image_height;
    let window = video_subsystem.window("flux render",
                                        image_width as u32,
                                        image_height as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator.create_texture_streaming(
        PixelFormatEnum::RGB24,
        image_width as u32,
        image_height as u32
        ).unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    // Set up manager ////////////////////////////////////////////////////////

    let image_builder = ImageBuilder::new();

    println!("Starting rendering manager");
    let mut manager = RenderManager::new(worker_handles, image_builder.sender());

    println!("Sending job to rendering manager");
    manager.schedule_job(s, c);

    // Set up GUI ////////////////////////////////////////////////////////////

    let mut copied_rows: Vec<bool> = (0..image_height).map(|_| false).collect();
    let mut finished = false;

    'running: loop {
        {
            if !finished {
                let img_ref = image_builder.get_image();
                let mut opt = img_ref.lock().unwrap();
                match opt.as_mut() {
                    None => (),
                    Some(img) => {
                        let mut num_skipped_rows = 0;
                        texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                            for y in 0..image_height {
                                if !copied_rows[y] {
                                    let ps = &img.pixels[y];

                                    if !ps.is_empty() {
                                        for (x, pixel) in ps.iter().enumerate() {
                                            let offset = y*pitch + x*3;
                                            buffer[offset] = (pixel.r * 255.99) as u8;
                                            buffer[offset + 1] = (pixel.g * 255.99) as u8;
                                            buffer[offset + 2] = (pixel.b * 255.99) as u8;
                                        }
                                        copied_rows[y] = true;
                                    }
                                } else {
                                    num_skipped_rows += 1;
                                }
                            }
                        }).unwrap();
                        if num_skipped_rows == image_height {
                            finished = true;
                        }
                    },
                }
            }
        }

        canvas.copy(&texture, None, None).expect("Render failed");
        canvas.present();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                _ => {}
            }
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    println!("Shutting down");

    if let Some(w) = local_worker {
        w.stop();
    }

    for w in net_workers {
        w.stop();
    }

    manager.stop();
    image_builder.stop();
}

#[derive(Debug)]
struct Config {
    network_workers: Vec<String>,
    use_local_worker: bool,
    sample_root: usize,
    max_depth: usize,
    rows_per_work_unit: usize,
}

fn config_from_args() -> Config {
    let app = App::new("flux")
        .author("Jonathan Daugherty <cygnus@foobox.com>")
        .about("Flux ray tracer")
        .arg(Arg::with_name("network_worker")
             .short("n")
             .long("node")
             .value_name("ADDRESS[:PORT]")
             .help("Render using the specified flux-node process at this address")
             .takes_value(true))
        .arg(Arg::with_name("depth")
             .short("d")
             .long("depth")
             .value_name("DEPTH")
             .help("Tracing depth")
             .takes_value(true))
        .arg(Arg::with_name("rowsperunit")
             .short("R")
             .long("rows")
             .value_name("COUNT")
             .help("Image rows per work unit")
             .takes_value(true))
        .arg(Arg::with_name("skip_local")
             .short("L")
             .help("Do not use the local host for rendering")
             .takes_value(false))
        .arg(Arg::with_name("sample_root")
             .short("r")
             .long("root")
             .help("Sample root")
             .takes_value(true));

    let ms = app.get_matches();
    let default_port = DEFAULT_PORT;
    let default_rows_per_work_unit = 50;

    Config {
        sample_root: match ms.value_of("sample_root") {
            None => DEFAULT_SAMPLE_ROOT,
            Some(r) => usize::from_str(r).unwrap(),
        },
        max_depth: match ms.value_of("depth") {
            None => DEFAULT_DEPTH,
            Some(d) => usize::from_str(d).unwrap(),
        },
        rows_per_work_unit: match ms.value_of("rowsperunit") {
            None => default_rows_per_work_unit,
            Some(r) => usize::from_str(r).unwrap(),
        },
        use_local_worker: match ms.occurrences_of("skip_local") {
            0 => true,
            _ => false,
        },
        network_workers: match ms.values_of("network_worker") {
            None => vec![],
            Some(v) => v.map(|s| String::from(s)).collect(),
        }
    }
}
