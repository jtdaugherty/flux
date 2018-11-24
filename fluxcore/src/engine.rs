
use rand::Rng;
use crossbeam::channel::{Sender, Receiver, unbounded};
use crossbeam::sync::WaitGroup;
use std::thread;

use scene::SceneData;
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

pub enum RenderEvent {
    ImageInfo { width: usize, height: usize },
    RowsReady(WorkUnitResult),
    RenderingFinished,
}

pub struct WorkUnitResult {
    pub work_unit: WorkUnit,
    pub rows: Vec<Vec<Color>>,
}

#[derive(Clone)]
#[derive(Copy)]
pub struct JobConfiguration {
    pub sample_root: usize,
    pub max_trace_depth: usize,
    pub rows_per_work_unit: usize,
}

pub fn work_units(j: &Job) -> Vec<WorkUnit> {
    if j.config.rows_per_work_unit <= 0 {
        panic!("Job row per work unit count invalid: {}",
               j.config.rows_per_work_unit);
    }

    let mut us = Vec::new();
    let mut i = 0;

    while i < j.scene_data.image_height - 1 {
        let remaining_rows = j.scene_data.image_height - i;
        let num_rows = std::cmp::min(j.config.rows_per_work_unit, remaining_rows);
        let u = WorkUnit {
            row_start: i,
            row_end: i + num_rows - 1,
            job_id: j.id,
        };
        us.push(u);
        i += num_rows;
    }

    us
}

// A job provides all the resources and configuration needed to render a
// scene.
#[derive(Clone)]
#[derive(Copy)]
pub struct Job {
    pub id: JobID,
    pub scene_data: SceneData,
    pub config: JobConfiguration,
}

pub struct RenderManager {
    job_id_allocator: JobIDAllocator,
    job_queue: Sender<Option<(Job, Sender<()>)>>,
    thread_handle: thread::JoinHandle<()>,
}

pub struct WorkerHandle {
    sender: Sender<Option<(Job, Receiver<Option<WorkUnit>>, Sender<RenderEvent>, WaitGroup)>>,
}

impl WorkerHandle {
    pub fn send(&self, j: Job, r: Receiver<Option<WorkUnit>>, s: Sender<RenderEvent>, wg: WaitGroup) {
        self.sender.send(Some((j, r, s, wg))).unwrap();
    }
}

pub struct JobHandle {
    waiter: Receiver<()>,
}

impl JobHandle {
    pub fn wait(&self) {
        self.waiter.recv().unwrap()
    }
}

impl RenderManager {
    pub fn new(workers: Vec<WorkerHandle>, result_sender: Sender<RenderEvent>) -> RenderManager {
        if workers.len() == 0 {
            panic!("RenderManager::new: must provide at least one worker handle");
        }

        let (s, r): (Sender<Option<(Job, Sender<()>)>>, Receiver<Option<(Job, Sender<()>)>>) = unbounded();

        let handle = thread::spawn(move || {
            println!("Render manager: awaiting job");

            while let Ok(Some((job, notify_done))) = r.recv() {
                println!("Render manager: got job {:?}", job.id);

                let info_event = RenderEvent::ImageInfo {
                    width: job.scene_data.image_width,
                    height: job.scene_data.image_height,
                };
                result_sender.send(info_event).unwrap();

                let (ws, wr) = unbounded();
                let wg = WaitGroup::new();

                for u in work_units(&job) {
                    ws.send(Some(u)).unwrap();
                }

                println!("Render manager: work queue ready, sending job to workers");

                workers.iter().for_each(|worker| {
                    ws.send(None).unwrap();
                    worker.send(job, wr.clone(), result_sender.clone(), wg.clone());
                });

                println!("Render manager: waiting for job completion");

                wg.wait();

                println!("Render manager: job complete");

                result_sender.send(RenderEvent::RenderingFinished).unwrap();

                notify_done.send(()).unwrap();
            }

            println!("Render manager: shutting down");
        });

        RenderManager {
            job_id_allocator: JobIDAllocator::new(),
            job_queue: s,
            thread_handle: handle,
        }
    }

    pub fn schedule_job(&mut self, scene_data: SceneData, config: JobConfiguration) -> JobHandle {
        let id = self.job_id_allocator.next();
        let (s, r): (Sender<()>, Receiver<()>) = unbounded();
        let j = Job {
            scene_data,
            config,
            id,
        };
        self.job_queue.send(Some((j, s))).unwrap();
        JobHandle {
            waiter: r
        }
    }

    pub fn stop(self) {
        self.job_queue.send(None).unwrap();
        self.thread_handle.join().unwrap();
    }
}

pub trait Worker {
    fn handle(&self) -> WorkerHandle;
    fn stop(self);
}

pub struct LocalWorker {
    sender: Sender<Option<(Job, Receiver<Option<WorkUnit>>, Sender<RenderEvent>, WaitGroup)>>,
    thread_handle: thread::JoinHandle<()>,
}

impl LocalWorker {
    pub fn new() -> LocalWorker {
        let (s, r): (Sender<Option<(Job, Receiver<Option<WorkUnit>>, Sender<RenderEvent>, WaitGroup)>>, Receiver<Option<(Job, Receiver<Option<WorkUnit>>, Sender<RenderEvent>, WaitGroup)>>) = unbounded();

        let handle = thread::spawn(move || {
            while let Ok(Some((job, recv_unit, send_result, wg))) = r.recv() {
                println!("Local worker: got job {:?}", job.id);
                // TODO build scene from scene data
                // TODO generate sample data
                while let Ok(Some(unit)) = recv_unit.recv() {
                    // TODO actually do the work and send the result
                    println!("Local worker: got work unit {:?}", unit);

                    let r = WorkUnitResult {
                        work_unit: unit,
                        rows: vec![],
                    };
                    let ev = RenderEvent::RowsReady(r);
                    send_result.send(ev).unwrap();
                }
                println!("Local worker finished job");
                drop(wg);
            }

            println!("Local worker shutting down");
        });

        LocalWorker {
            sender: s,
            thread_handle: handle,
        }
    }
}

impl Worker for LocalWorker {
    fn handle(&self) -> WorkerHandle {
        WorkerHandle {
            sender: self.sender.clone(),
        }
    }

    fn stop(self) {
        self.sender.send(None).unwrap();
        self.thread_handle.join().unwrap();
    }
}

pub struct ConsoleResultReporter {
    sender: Sender<RenderEvent>,
}

impl ConsoleResultReporter {
    pub fn new() -> ConsoleResultReporter {
        let (s, r): (Sender<RenderEvent>, Receiver<RenderEvent>) = unbounded();

        thread::spawn(move || {
            while let Ok(result) = r.recv() {
                match result {
                    RenderEvent::ImageInfo { width, height } => {
                        println!("ConsoleResultReporter: image {} x {} pixels",
                                 width, height);
                    },
                    RenderEvent::RowsReady(unit_result) => {
                        println!("ConsoleResultReporter: image fragment done, {} rows",
                                 unit_result.work_unit.row_end - unit_result.work_unit.row_start + 1);
                    },
                    RenderEvent::RenderingFinished => {
                        println!("ConsoleResultReporter: rendering finished");
                    }
                }
            }
        });

        ConsoleResultReporter {
            sender: s,
        }
    }

    pub fn sender(&self) -> Sender<RenderEvent> {
        self.sender.clone()
    }
}
