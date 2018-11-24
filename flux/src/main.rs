
extern crate fluxcore;

use fluxcore::engine::*;
use fluxcore::scene::SceneData;

fn main() {
    let c = JobConfiguration {
        rows_per_work_unit: 100,
        max_trace_depth: 1,
        sample_root: 1,
    };
    let s = SceneData {
        image_width: 800,
        image_height: 600,
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
