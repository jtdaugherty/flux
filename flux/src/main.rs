
extern crate fluxcore;

use fluxcore::engine::*;
use fluxcore::scene::Scene;

fn main() {
    let c = JobConfiguration {
        rows_per_work_unit: 100,
        max_trace_depth: 1,
        sample_root: 1,
    };
    let s = Scene {
        image_width: 800,
        image_height: 600,
    };
    let j = Job {
        config: c,
        scene: s,
    };
    let mut a = JobIDAllocator::new();
    let id = a.next();

    let us = work_units(id, &j);

    for u in us {
        println!("{:?}", u);
    }
}
