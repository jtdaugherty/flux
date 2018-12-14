
use std::time::Duration;
use std::str::FromStr;
use std::process::exit;

use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::image::{INIT_PNG, INIT_JPG};

use fluxcore::manager::*;
use fluxcore::workers::{LocalWorker, NetworkWorker};
use fluxcore::job::JobConfiguration;
use fluxcore::scene::*;

use clap::{App, Arg};

use std::fs::File;

const DEFAULT_SAMPLE_ROOT: usize = 1;
const DEFAULT_DEPTH: usize = 5;

fn main() {
    // Get the configuration from the command-line arguments
    let config = config_from_args();

    // Load the YAML scene file
    let scene_file = File::open(config.input_filename).unwrap();
    let s: SceneData = serde_yaml::from_reader(scene_file).unwrap();

    // Check that we have at least one worker
    if !config.use_local_worker && config.network_workers.is_empty() {
        println!("No workers specified, exiting");
        return;
    }

    let mut worker_handles: Vec<WorkerHandle> = vec![];
    let mut local_worker: Option<LocalWorker> = None;
    let mut net_workers: Vec<NetworkWorker> = vec![];

    // Start local worker, if any
    if config.use_local_worker {
        let worker = LocalWorker::new(config.num_threads);
        println!("Local worker ready, info:");
        worker.info().print();
        worker_handles.push(worker.handle());
        local_worker = Some(worker);
    }

    // Connect to network workers, if any
    for endpoint in config.network_workers {
        println!("Connecting to {}", &endpoint);
        match NetworkWorker::new(&endpoint) {
            Err(e) => {
                println!("Could not connect network node '{}': {}", endpoint, e);
                exit(1);
            }
            Ok(worker) => {
                println!("Network worker ready, info:");
                worker.info().print();

                worker_handles.push(worker.handle());
                net_workers.push(worker);
            }
        }
    }

    // Start an image accumulator thread
    let image_builder = ImageBuilder::new();

    // Start the rendering manager
    println!("Starting rendering manager");
    let mut manager = RenderManager::new(worker_handles, image_builder.sender());

    // Build a job configuration from the local config
    let jobcfg = JobConfiguration {
        rows_per_work_unit: config.rows_per_work_unit,
        max_trace_depth: config.max_depth,
        sample_root: config.sample_root,
    };

    // Submit the job to the rendering manager
    println!("Sending job to rendering manager");
    let job = manager.schedule_job(&s, jobcfg);

    if config.show_live_preview {
        // If the live preview was requested, create an SDL window and
        // update it from the image accumulator
        show_preview(&job, &s, &image_builder);
    } else {
        // Else the live preview was not requested, so just block until
        // the job completes.
        job.wait();
    }

    println!("Shutting down");

    if let Some(w) = local_worker {
        // Stop the local worker
        w.stop();
    }

    for w in net_workers {
        // Notify network workers that we're done and disconnect
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
    input_filename: String,
    show_live_preview: bool,
    num_threads: usize,
}

fn config_from_args() -> Config {
    let app = App::new("flux")
        .author("Jonathan Daugherty <cygnus@foobox.com>")
        .about("Flux ray tracer")
        .arg(Arg::with_name("scene_file")
             .index(1)
             .required(true))
        .arg(Arg::with_name("network_worker")
             .short("n")
             .long("node")
             .value_name("ADDRESS[:PORT]")
             .help("Render using the specified flux-node process at this address")
             .multiple(true)
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
        .arg(Arg::with_name("show_preview")
             .short("g")
             .help("Show a live graphical preview window during rendering")
             .takes_value(false))
        .arg(Arg::with_name("threads")
             .short("t")
             .long("threads")
             .help("Number of rendering threads for the local worker (defaults to number of logical CPUs)")
             .takes_value(true))
        .arg(Arg::with_name("sample_root")
             .short("r")
             .long("root")
             .help("Sample root")
             .takes_value(true));

    let ms = app.get_matches();
    let default_rows_per_work_unit = 50;

    Config {
        show_live_preview: ms.occurrences_of("show_preview") > 0,
        input_filename: match ms.value_of("scene_file") {
            None => panic!("Scene filename is required"),
            Some(f) => String::from(f),
        },
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
        },
        num_threads: match ms.value_of("threads") {
            None => num_cpus::get(),
            Some(t) => usize::from_str(t).unwrap(),
        },
    }
}

fn show_preview(job: &JobHandle, s: &SceneData, image_builder: &ImageBuilder) {
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
                        job.cancel();
                        break 'running
                    },
                _ => {}
            }
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
