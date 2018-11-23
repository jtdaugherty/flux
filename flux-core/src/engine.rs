
use rand::Rng;

use scene::Scene;
use color::Color;

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Eq)]
pub struct JobID(usize, usize);

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

#[derive(Debug)]
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
    pub fn new(_workers: Vec<Box<Worker>>) -> RenderManager {
        RenderManager {
            job_id_allocator: JobIDAllocator::new(),
        }
    }

    pub fn schedule_job(&mut self, _j: Job) -> JobID {
        self.job_id_allocator.next()
    }

    pub fn cancel_job(&mut self, _id: JobID) {
    }
}

pub trait Worker {
    fn set_job(&mut self, j: Job, id: JobID);
    fn render(&mut self, u: WorkUnit) -> bool;
}

pub trait RowHandler {
    fn row_ready(&mut self, id: JobID, row_index: usize, v: Vec<Color>);
}

pub struct LocalWorker {
    current_job: Option<(JobID, Job)>,
    row_handler: Box<RowHandler>,
    // Need to store a work queue of work units that gets populated by
    // render() and gets consumed by worker thread
}

impl LocalWorker {
    fn new(handler: Box<RowHandler>) -> LocalWorker {
        LocalWorker {
            current_job: None,
            row_handler: handler,
        }
    }
}

impl Worker for LocalWorker {
    fn set_job(&mut self, j: Job, id: JobID) {
        self.current_job = Some((id, j));
    }

    fn render(&mut self, u: WorkUnit) -> bool {
        match &self.current_job {
            Some((j_id, j)) => {
                if *j_id == u.job_id {
                    println!("LocalWorker: got work unit {:?} for current job", u);
                    // TODO: actually render the requested row range,
                    // then feed each row to the handler
                    // self.row_handler.row_ready(u.job_id, 0, vec![]);
                    true
                } else {
                    false
                }
            },
            None => false
        }
    }
}
