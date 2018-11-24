
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
    let c2 = JobConfiguration {
        rows_per_work_unit: 200,
        max_trace_depth: 1,
        sample_root: 1,
    };
    let s2 = SceneData {
        image_width: 1024,
        image_height: 768,
    };

    let worker = LocalWorker::new();
    let reporter = ConsoleResultReporter::new();
    let mut manager = RenderManager::new(vec![worker.sender()], reporter.sender());

    let job = manager.schedule_job(s, c);
    let job2 = manager.schedule_job(s2, c2);

    job.wait();
    job2.wait();

    manager.stop();
    worker.stop();
}
