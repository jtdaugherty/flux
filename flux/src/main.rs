
extern crate fluxcore;
extern crate nalgebra;

use nalgebra::{Point3, Vector3};

use fluxcore::manager::*;
use fluxcore::job::JobConfiguration;
use fluxcore::scene::*;
use fluxcore::shapes::*;
use fluxcore::color::Color;

fn main() {
    let c = JobConfiguration {
        rows_per_work_unit: 100,
        max_trace_depth: 1,
        sample_root: 1,
    };
    let s = SceneData {
        scene_name: String::from("test_scene"),
        output_settings: OutputSettings {
            image_width: 400,
            image_height: 400,
            pixel_size: 1.0,
        },
        camera_settings: CameraSettings::new(
                             Point3::new(0.0, 1.5, -3.0),
                             Point3::new(0.0, 0.0, 0.0),
                             Vector3::new(0.0, 1.0, 0.0)),
        camera_data: CameraData {
            zoom_factor: 1.0,
            view_plane_distance: 100.0,
            focal_distance: 20.0,
            lens_radius: 0.0,
        },
        background: Color::black(),
        shapes: vec![
            ShapeData {
                shape_type: ShapeType::Sphere,
                content: ShapeContent {
                    sphere: Sphere {
                        center: Point3::new(0.0, 1.0, 0.0),
                        radius: 1.0,
                        color: Color::new(1.0, 0.0, 0.0),
                    }
                },
            },
            ShapeData {
                shape_type: ShapeType::Plane,
                content: ShapeContent {
                    plane: Plane {
                        point: Point3::new(0.0, 0.0, 0.0),
                        normal: Vector3::new(0.0, 1.0, 0.0),
                        color: Color::new(0.0, 0.0, 1.0),
                    }
                },
            },
        ],
    };

    println!("Starting local worker");
    let worker = LocalWorker::new();
    let image_builder = ImageBuilder::new();

    println!("Starting rendering manager");
    let mut manager = RenderManager::new(vec![worker.handle()], image_builder.sender());

    println!("Sending job to rendering manager");
    let job = manager.schedule_job(s, c);

    println!("Awaiting job completion");
    job.wait();

    println!("Shutting down");
    manager.stop();
    worker.stop();
    image_builder.stop();
}
