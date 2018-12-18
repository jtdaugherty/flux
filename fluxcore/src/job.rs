
use rand::Rng;

use crate::scene::SceneData;

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Eq)]
#[derive(Clone)]
#[derive(Copy)]
#[derive(Serialize, Deserialize)]
pub struct JobID(usize, usize);

pub struct JobIDAllocator {
    allocator_id: usize,
    next_id: usize,
}

impl JobIDAllocator {
    pub fn new() -> Self {
        let mut trng = rand::thread_rng();

        Self {
            allocator_id: trng.gen(),
            next_id: 0,
        }
    }

    pub fn next_id(&mut self) -> JobID {
        let j = JobID(self.allocator_id, self.next_id);
        self.next_id += 1;
        j
    }
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(Copy)]
#[derive(Serialize, Deserialize)]
pub struct WorkUnit {
    pub row_start: usize,
    pub row_end: usize,
    pub job_id: JobID,
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Serialize, Deserialize, Debug)]
pub struct JobConfiguration {
    pub sample_root: usize,
    pub max_trace_depth: usize,
    pub rows_per_work_unit: usize,
}

// A job provides all the resources and configuration needed to render a
// scene.
#[derive(Clone)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Job {
    pub id: JobID,
    pub scene_data: SceneData,
    pub config: JobConfiguration,
}

impl Job {
    pub fn work_units(&self) -> Vec<WorkUnit> {
        if self.config.rows_per_work_unit == 0 {
            panic!("Job row per work unit count invalid: {}",
                   self.config.rows_per_work_unit);
        }

        let mut us = Vec::new();
        let mut i = 0;

        while i < self.scene_data.output_settings.image_height - 1 {
            let remaining_rows = self.scene_data.output_settings.image_height - i;
            let num_rows = std::cmp::min(self.config.rows_per_work_unit, remaining_rows);
            let u = WorkUnit {
                row_start: i,
                row_end: i + num_rows - 1,
                job_id: self.id,
            };
            us.push(u);
            i += num_rows;
        }

        us
    }
}
