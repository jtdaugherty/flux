
use crossbeam::channel::{Sender, Receiver, unbounded};
use crossbeam::sync::WaitGroup;
use std::fs::File;
use std::thread;

use scene::SceneData;
use color::Color;
use image::Image;
use job::{JobConfiguration, Job, JobIDAllocator, WorkUnit};

const ENGINE_DEBUG: bool = false;

fn d_println(s: String) {
    if ENGINE_DEBUG {
        println!("{}", s);
    }
}

pub enum RenderEvent {
    ImageInfo { scene_name: String, width: usize, height: usize },
    RowsReady(WorkUnitResult),
    RenderingFinished,
}

pub struct WorkUnitResult {
    pub work_unit: WorkUnit,
    pub rows: Vec<Vec<Color>>,
}

pub struct RenderManager {
    job_id_allocator: JobIDAllocator,
    job_queue: Sender<Option<(Job, Sender<()>)>>,
    thread_handle: thread::JoinHandle<()>,
}

pub struct WorkerHandle {
    sender: Sender<Option<(Job, Receiver<Option<WorkUnit>>, Sender<Option<RenderEvent>>, WaitGroup)>>,
}

impl WorkerHandle {
    pub fn send(&self, j: Job, r: Receiver<Option<WorkUnit>>, s: Sender<Option<RenderEvent>>, wg: WaitGroup) {
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
    pub fn new(workers: Vec<WorkerHandle>, result_sender: Sender<Option<RenderEvent>>) -> RenderManager {
        if workers.len() == 0 {
            panic!("RenderManager::new: must provide at least one worker handle");
        }

        let (s, r): (Sender<Option<(Job, Sender<()>)>>, Receiver<Option<(Job, Sender<()>)>>) = unbounded();

        let handle = thread::spawn(move || {
            d_println(format!("Render manager: awaiting job"));

            while let Ok(Some((job, notify_done))) = r.recv() {
                d_println(format!("Render manager: got job {:?}", job.id));

                let info_event = RenderEvent::ImageInfo {
                    scene_name: job.scene_data.scene_name.clone(),
                    width: job.scene_data.output_settings.image_width,
                    height: job.scene_data.output_settings.image_height,
                };
                result_sender.send(Some(info_event)).unwrap();

                let (ws, wr) = unbounded();
                let wg = WaitGroup::new();

                for u in job.work_units() {
                    ws.send(Some(u)).unwrap();
                }

                d_println(format!("Render manager: work queue ready, sending job to workers"));

                workers.iter().for_each(|worker| {
                    ws.send(None).unwrap();
                    worker.send(job.clone(), wr.clone(), result_sender.clone(), wg.clone());
                });

                d_println(format!("Render manager: waiting for job completion"));

                wg.wait();

                d_println(format!("Render manager: job complete"));

                result_sender.send(Some(RenderEvent::RenderingFinished)).unwrap();

                notify_done.send(()).unwrap();
            }

            d_println(format!("Render manager: shutting down"));
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
    sender: Sender<Option<(Job, Receiver<Option<WorkUnit>>, Sender<Option<RenderEvent>>, WaitGroup)>>,
    thread_handle: thread::JoinHandle<()>,
}

impl LocalWorker {
    pub fn new() -> LocalWorker {
        let (s, r): (Sender<Option<(Job, Receiver<Option<WorkUnit>>, Sender<Option<RenderEvent>>, WaitGroup)>>, Receiver<Option<(Job, Receiver<Option<WorkUnit>>, Sender<Option<RenderEvent>>, WaitGroup)>>) = unbounded();

        let handle = thread::spawn(move || {
            while let Ok(Some((job, recv_unit, send_result, wg))) = r.recv() {
                d_println(format!("Local worker: got job {:?}", job.id));
                // TODO build scene from scene data
                // TODO generate sample data
                while let Ok(Some(unit)) = recv_unit.recv() {
                    // TODO actually do the work and send the result
                    d_println(format!("Local worker: got work unit {:?}", unit));

                    let r = WorkUnitResult {
                        work_unit: unit,
                        rows: vec![],
                    };
                    let ev = RenderEvent::RowsReady(r);
                    send_result.send(Some(ev)).unwrap();
                }
                d_println(format!("Local worker finished job"));
                drop(wg);
            }

            d_println(format!("Local worker shutting down"));
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
    sender: Sender<Option<RenderEvent>>,
}

impl ConsoleResultReporter {
    pub fn new() -> ConsoleResultReporter {
        let (s, r): (Sender<Option<RenderEvent>>, Receiver<Option<RenderEvent>>) = unbounded();

        thread::spawn(move || {
            while let Ok(Some(result)) = r.recv() {
                match result {
                    RenderEvent::ImageInfo { scene_name, width, height } => {
                        println!("ConsoleResultReporter: scene: {}", scene_name);
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

    pub fn sender(&self) -> Sender<Option<RenderEvent>> {
        self.sender.clone()
    }
}

pub struct ImageBuilder {
    sender: Sender<Option<RenderEvent>>,
    thread_handle: thread::JoinHandle<()>,
}

impl ImageBuilder {
    pub fn new() -> ImageBuilder {
        let (s, r): (Sender<Option<RenderEvent>>, Receiver<Option<RenderEvent>>) = unbounded();

        let thread_handle = thread::spawn(move || {
            let (scene_name, width, height) = match r.recv() {
                Ok(Some(RenderEvent::ImageInfo { scene_name, width, height } )) => (scene_name, width, height),
                _ => panic!("ImageBuilder: got unexpected message"),
            };

            println!("ImageBuilder: image {} x {} pixels",
                     width, height);

            let mut img = Image::new(width, height);

            while let Ok(Some(result)) = r.recv() {
                match result {
                    RenderEvent::RowsReady(unit_result) => {
                        println!("ImageBuilder: image fragment done, {} rows",
                                 unit_result.work_unit.row_end - unit_result.work_unit.row_start + 1);

                        for (i, row) in unit_result.rows.into_iter().enumerate() {
                            img.set_row(i + unit_result.work_unit.row_start, row);
                        }
                    },
                    RenderEvent::RenderingFinished => {
                        println!("ImageBuilder: rendering finished");
                        let filename = scene_name.clone() + ".ppm";
                        let mut output_file = File::create(filename).unwrap();
                        img.write(&mut output_file);
                    },
                    _ => panic!("ImageBuilder: got unexpected message"),
                }
            }
        });

        ImageBuilder {
            sender: s,
            thread_handle,
        }
    }

    pub fn sender(&self) -> Sender<Option<RenderEvent>> {
        self.sender.clone()
    }

    pub fn stop(self) {
        self.sender.send(None).unwrap();
        self.thread_handle.join().unwrap();
    }
}
