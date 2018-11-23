
use rand::Rng;

use scene::Scene;
use color::Color;

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Eq)]
#[derive(Clone)]
#[derive(Copy)]
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
#[derive(Clone)]
#[derive(Copy)]
pub struct WorkUnit {
    row_start: usize,
    row_end: usize,
    job_id: JobID,
}

pub struct WorkUnitResult {
    pub work_unit: WorkUnit,
    pub rows: Vec<Vec<Color>>,
}

pub struct JobConfiguration {
    pub sample_root: usize,
    pub max_trace_depth: usize,
    pub rows_per_work_unit: usize,
}

pub fn work_units(id: JobID, j: &Job) -> Vec<WorkUnit> {
    if j.config.rows_per_work_unit <= 0 {
        panic!("Job row per work unit count invalid: {}",
               j.config.rows_per_work_unit);
    }

    let mut us = Vec::new();
    let mut i = 0;

    while i < j.scene.image_height - 1 {
        let remaining_rows = j.scene.image_height - i;
        let num_rows = std::cmp::min(j.config.rows_per_work_unit, remaining_rows);
        let u = WorkUnit {
            row_start: i,
            row_end: i + num_rows - 1,
            job_id: id,
        };
        us.push(u);
        i += num_rows;
        println!("{}", i);
    }

    us
}

// A job provides all the resources and configuration needed to render a
// scene.
pub struct Job {
    pub scene: Scene,
    pub config: JobConfiguration,
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
    // Set the worker's current job.
    fn set_job(&mut self, j: Job, id: JobID);

    // Schedules the work unit to be rendered. Returns whether the
    // scheduling succeeded. Fails if the work unit is not for the
    // current job.
    fn render(&mut self, u: WorkUnit) -> bool;
}

pub trait ResultHandler {
    // Called by a Worker when a work unit has been finished.
    fn work_unit_finished(&mut self, r: WorkUnitResult);
}

pub struct LocalWorker {
    current_job: Option<(JobID, Job)>,
    result_handler: Box<ResultHandler>,
    // Need to store a work queue of work units that gets populated by
    // render() and gets consumed by worker thread
}

impl LocalWorker {
    fn new(handler: Box<ResultHandler>) -> LocalWorker {
        LocalWorker {
            current_job: None,
            result_handler: handler,
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
                    // self.result_handler.work_unit_finished(...);
                    true
                } else {
                    false
                }
            },
            None => false
        }
    }
}
