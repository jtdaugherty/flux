
extern crate fluxcore;
extern crate nalgebra;

use nalgebra::{Point3};

use fluxcore::manager::*;
use fluxcore::job::JobConfiguration;
use fluxcore::scene::*;
use fluxcore::color::Color;

fn main() {
    let c = JobConfiguration {
        rows_per_work_unit: 100,
        max_trace_depth: 1,
        sample_root: 1,
    };
    let s = SceneData {
        output_settings: OutputSettings {
            image_width: 800,
            image_height: 600,
            pixel_size: 1.0,
        },
        background: Color::black(),
        shapes: vec![
            ShapeData {
                shape_type: ShapeType::Sphere,
                content: ShapeContent {
                    sphere: Sphere {
                        center: Point3::new(0.0, 0.0, 0.0),
                        radius: 1.0,
                        color: Color::new(1.0, 1.0, 1.0),
                    }
                },
            },
        ],
    };

    println!("Starting local worker");
    let worker = LocalWorker::new();
    let reporter = ConsoleResultReporter::new();

    println!("Starting rendering manager");
    let mut manager = RenderManager::new(vec![worker.handle()], reporter.sender());

    println!("Sending job to rendering manager");
    let job = manager.schedule_job(s, c);

    println!("Awaiting job completion");
    job.wait();

    println!("Shutting down");
    manager.stop();
    worker.stop();
}
