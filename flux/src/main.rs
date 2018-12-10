
extern crate fluxcore;
extern crate nalgebra;
extern crate sdl2;

use std::time::Duration;

use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::image::{INIT_PNG, INIT_JPG};

use nalgebra::{Point3, Vector3};

use fluxcore::manager::*;
use fluxcore::job::JobConfiguration;
use fluxcore::scene::*;
use fluxcore::shapes::*;
use fluxcore::color::Color;

fn main() {
    let c = JobConfiguration {
        rows_per_work_unit: 100,
        max_trace_depth: 10,
        sample_root: 10,
    };
    let s = SceneData {
        scene_name: String::from("test_scene"),
        output_settings: OutputSettings {
            image_width: 800,
            image_height: 600,
            pixel_size: 0.5,
        },
        camera_settings: CameraSettings::new(
                             Point3::new(0.0, 1.5, -9.0),
                             Point3::new(0.0, 1.0, 0.0),
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
                center: Point3::new(-3.0, 1.0, -4.0),
                radius: 1.0,
                invert: false,
                material: MaterialData::Matte(MatteData {
                    diffuse_coefficient: 1.0,
                    ambient_color: Color::white(),
                    diffuse_color: Color::new(0.0, 0.7, 0.6),
                })
            }),
            ShapeData::Sphere(SphereData {
                center: Point3::new(1.5, 1.0, 2.0),
                radius: 1.0,
                invert: false,
                material: MaterialData::Reflective(ReflectiveData {
                    reflect_amount: 0.9,
                    reflect_color: Color::new(0.9, 1.0, 0.9),
                })
            }),
            ShapeData::Sphere(SphereData {
                center: Point3::new(3.0, 1.0, 4.0),
                radius: 1.0,
                invert: false,
                material: MaterialData::Matte(MatteData {
                    diffuse_coefficient: 1.0,
                    ambient_color: Color::white(),
                    diffuse_color: Color::new(0.0, 0.1, 0.9),
                })
            }),
            ShapeData::Sphere(SphereData {
                center: Point3::new(4.5, 1.0, 6.0),
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

    //////////////////////////////////////////////////////////////////////////

    println!("Starting local worker");
    let worker = LocalWorker::new();
    let image_builder = ImageBuilder::new();

    println!("Starting rendering manager");
    let mut manager = RenderManager::new(vec![worker.handle()], image_builder.sender());

    println!("Sending job to rendering manager");
    manager.schedule_job(s, c);

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

                                    if ps.len() > 0 {
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
    manager.stop();
    worker.stop();
    image_builder.stop();
}
