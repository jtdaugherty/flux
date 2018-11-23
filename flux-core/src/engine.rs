
use rand::Rng;

use scene::Scene;

struct JobID(usize, usize);

pub struct JobIDAllocator {
    allocator_id: usize,
    next_id: usize,
}

impl JobIDAllocator {
    pub fn new() -> JobIDAllocator {
        let mut trng = rand::thread_rng();

        JobIDAllocator {
            allocator_id: trng.gen(),
            next_id: 0,
        }
    }

    pub fn next(&mut self) -> JobID {
        let j = JobID(self.allocator_id, self.next_id);
        self.next_id += 1;
        j
    }
}

pub struct WorkUnit {
    row_start: usize,
    row_end: usize,
    job_id: JobID,
}

pub struct JobConfiguration {
    sample_root: usize,
    max_trace_depth: usize,
    rows_per_work_unit: usize,
}

// A job provides all the resources and configuration needed to render a
// scene.
pub struct Job {
    scene: Scene,
    config: JobConfiguration,
}

pub struct RenderManager {
    job_id_allocator: JobIDAllocator,

    // Also need state for:
    //
    // Worker vector
    // Thread handle(s) for any threads we spawned in the constructor
    // Work unit queue that threads will take from when they need work
    //
    // Need to make it possible for different workers to be working on
    // different jobs, for full generality
}

impl RenderManager {
    pub fn new(workers: Vec<Box<Worker>>) -> RenderManager {
        RenderManager {
            job_id_allocator: JobIDAllocator::new(),
        }
    }

    pub fn schedule_job(&mut self, j: Job) -> JobID {
        self.job_id_allocator.next()
    }

    pub fn cancel_job(&mut self, id: JobID) {
    }
}

pub trait Worker {
    fn set_job(&mut self, j: Job, id: JobID);
    fn render(&mut self, u: WorkUnit);
}

pub struct LocalWorker {
}
