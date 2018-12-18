
use crossbeam::channel::{Sender, Receiver, bounded, unbounded};
use crossbeam::sync::WaitGroup;
use crossbeam::SendError;
use std::fs::File;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::scene::{SceneData};
use crate::color::Color;
use crate::image::Image;
use crate::job::{JobConfiguration, Job, JobID, JobIDAllocator, WorkUnit};
use crate::debug::d_println;

#[derive(Serialize, Deserialize)]
pub enum RenderEvent {
    RenderingStarted { job_id: JobID, start_time: SystemTime, },
    ImageInfo { scene_name: String, width: usize, height: usize },
    RowsReady(WorkUnitResult),
    RenderingFinished { end_time: SystemTime },
}

#[derive(Serialize, Deserialize)]
pub struct WorkUnitResult {
    pub work_unit: WorkUnit,
    pub rows: Vec<Vec<Color>>,
}

pub struct RenderManager {
    job_id_allocator: JobIDAllocator,
    job_queue: Sender<ScheduledJob>,
    thread_handle: thread::JoinHandle<()>,
}

pub type WorkerRequest = Option<(Box<Job>, Receiver<WorkUnit>, Sender<Option<RenderEvent>>, WaitGroup)>;
type ScheduledJob = Option<(Job, Sender<()>, Receiver<()>, Sender<Option<RenderEvent>>)>;

pub struct WorkerHandle {
    sender: Sender<WorkerRequest>,
}

impl WorkerHandle {
    pub fn new(sender: Sender<WorkerRequest>) -> Self {
        Self {
            sender,
        }
    }

    pub fn send(&self, j: Box<Job>, r: Receiver<WorkUnit>, s: Sender<Option<RenderEvent>>, wg: WaitGroup) -> Result<(), SendError<WorkerRequest>> {
        self.sender.send(Some((j, r, s, wg)))
    }
}

pub struct JobHandle {
    job_id: JobID,
    waiter: Receiver<()>,
    canceller: Sender<()>,
}

impl JobHandle {
    pub fn wait(&self) {
        self.waiter.recv().unwrap()
    }

    pub fn cancel(&self) {
        d_println(format!("Job cancellation request for {:?}", self.job_id));
        self.canceller.send(()).unwrap();
    }
}

impl RenderManager {
    pub fn new(workers: Vec<WorkerHandle>) -> Self {
        if workers.is_empty() {
            panic!("RenderManager::new: must provide at least one worker handle");
        }

        let (s, r): (Sender<ScheduledJob>, Receiver<ScheduledJob>) = unbounded();

        let handle = thread::Builder::new().name("RenderManager".to_string()).spawn(move || {
            d_println(format!("Render manager: awaiting job"));

            while let Ok(Some((job, notify_done, notify_cancel, result_sender))) = r.recv() {
                d_println(format!("Render manager: got job {:?}", job.id));

                let info_event = RenderEvent::ImageInfo {
                    scene_name: job.scene_data.scene_name.clone(),
                    width: job.scene_data.output_settings.image_width,
                    height: job.scene_data.output_settings.image_height,
                };

                match result_sender.send(Some(info_event)) {
                    Ok(_) => (),
                    Err(_) => {
                        d_println(format!("RenderManager advancing to next job due to info event send error"));
                        continue;
                    }
                }

                let (ws, wr) = bounded(1);
                let wg = WaitGroup::new();
                let units = job.work_units().into_iter();
                let wu_queue = Arc::new(Mutex::new(CancellableIterator::new(units)));

                let wu_queue_cancel = Arc::clone(&wu_queue);
                thread::Builder::new().name(format!("Cancel listener for {:?}", job.id)).spawn(move || {
                    d_println(format!("Cancel listener waiting for cancel message"));
                    match notify_cancel.recv() {
                        Ok(_) => (),
                        Err(_) => {
                            return;
                        }
                    }
                    d_println(format!("Cancel listener got cancellation"));
                    wu_queue_cancel.lock().unwrap().cancel();
                }).unwrap();

                let wu_queue_read = Arc::clone(&wu_queue);
                let job_id = job.id.clone();
                thread::Builder::new().name(format!("Work queue for {:?}", job.id)).spawn(move || {
                    d_println(format!("Work queue producer starting"));
                    loop {
                        let mut q = wu_queue_read.lock().unwrap();
                        match q.next() {
                            None => {
                                d_println(format!("Work unit iterator cancelled or empty, quitting"));
                                return;
                            },
                            Some(i) => {
                                d_println(format!("Work item ready, adding to queue"));
                                match ws.send(i.clone()) {
                                    Ok(_) => (),
                                    Err(_) => {
                                        d_println(format!("Work queue for {:?} exiting due to send failure", job_id));
                                        return;
                                    },
                                }
                            }
                        }
                    }
                }).unwrap();

                d_println(format!("Render manager: work queue ready, sending job to workers"));

                let start_time = SystemTime::now();
                let start_event = RenderEvent::RenderingStarted { job_id: job.id, start_time, };

                match result_sender.send(Some(start_event)) {
                    Ok(_) => (),
                    Err(_) => {
                        d_println(format!("RenderManager advancing to next job due to start event send error"));
                        continue;
                    }
                }

                workers.iter().for_each(|worker| {
                    let job_boxed = Box::new(job.clone());
                    // Note that we panic on send failures here because
                    // a worker that is down is bad enough that we can't
                    // recover
                    worker.send(job_boxed, wr.clone(), result_sender.clone(), wg.clone()).unwrap();
                });

                d_println(format!("Render manager: waiting for job completion or cancellation"));

                wg.wait();

                d_println(format!("Render manager: all workers done"));

                let end_time = SystemTime::now();
                match result_sender.send(Some(RenderEvent::RenderingFinished { end_time, })) {
                    Ok(_) => (),
                    Err(_) => {
                        d_println(format!("RenderManager advancing to next job due to result_sender send error"));
                        continue;
                    }
                }

                match notify_done.send(()) {
                    Ok(_) => (),
                    Err(_) => {
                        d_println(format!("RenderManager advancing to next job due to notify_done send error"));
                        continue;
                    }
                }
            }

            d_println(format!("Render manager: shutting down"));
        }).unwrap();

        Self {
            job_id_allocator: JobIDAllocator::new(),
            job_queue: s,
            thread_handle: handle,
        }
    }

    pub fn schedule_job(&mut self, scene_data: &SceneData, config: JobConfiguration, result_sender: Sender<Option<RenderEvent>>) -> JobHandle {
        let id = self.job_id_allocator.next_id();
        let (s, r): (Sender<()>, Receiver<()>) = unbounded();
        let (cs, cr): (Sender<()>, Receiver<()>) = unbounded();
        let j = Job {
            scene_data: scene_data.clone(),
            config,
            id,
        };
        self.job_queue.send(Some((j, s, cr, result_sender))).unwrap();
        JobHandle {
            job_id: id,
            waiter: r,
            canceller: cs,
        }
    }

    pub fn stop(self) {
        self.job_queue.send(None).ok();
        self.thread_handle.join().ok();
    }
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct WorkerInfo {
    pub num_threads: usize,
}

impl WorkerInfo {
    pub fn print(&self) {
        println!("Threads: {}", self.num_threads);
    }
}

pub trait Worker {
    fn handle(&self) -> WorkerHandle;
    fn stop(self);
    fn info(&self) -> WorkerInfo;
}

pub struct ConsoleResultReporter {
    sender: Sender<Option<RenderEvent>>,
}

impl ConsoleResultReporter {
    pub fn new() -> Self {
        let (s, r): (Sender<Option<RenderEvent>>, Receiver<Option<RenderEvent>>) = unbounded();

        thread::Builder::new().name("ConsoleResultReporter".to_string()).spawn(move || {
            while let Ok(Some(result)) = r.recv() {
                match result {
                    RenderEvent::RenderingStarted { job_id, start_time, } => {
                        println!("ConsoleResultReporter: job {:?} started at {:?}", job_id, start_time);
                    },
                    RenderEvent::ImageInfo { scene_name, width, height } => {
                        println!("ConsoleResultReporter: scene: {}", scene_name);
                        println!("ConsoleResultReporter: image {} x {} pixels",
                                 width, height);
                    },
                    RenderEvent::RowsReady(unit_result) => {
                        println!("ConsoleResultReporter: image fragment done, {} rows",
                                 unit_result.work_unit.row_end - unit_result.work_unit.row_start + 1);
                    },
                    RenderEvent::RenderingFinished { end_time, } => {
                        println!("ConsoleResultReporter: rendering finished at {:?}", end_time);
                    }
                }
            }
        }).unwrap();

        Self {
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
    image: Arc<Mutex<Option<Image>>>,
}

impl ImageBuilder {
    pub fn new() -> Self {
        let (s, r): (Sender<Option<RenderEvent>>, Receiver<Option<RenderEvent>>) = unbounded();
        let img_ref = Arc::new(Mutex::new(None));
        let img_ref_thread = img_ref.clone();

        let thread_handle = thread::Builder::new().name("ImageBuilder".to_string()).spawn(move || {
            let (scene_name, width, height) = match r.recv() {
                Ok(Some(RenderEvent::ImageInfo { scene_name, width, height } )) => (scene_name, width, height),
                _ => {
                    d_println(format!("ImageBuilder: got unexpected message"));
                    return;
                },
            };

            d_println(format!("ImageBuilder: image {} x {} pixels", width, height));

            let start_time = match r.recv() {
                Ok(Some(RenderEvent::RenderingStarted { start_time, .. })) => start_time,
                _ => {
                    d_println(format!("ImageBuilder: got unexpected message when expecting render start message"));
                    return;
                }
            };

            {
                let mut img = img_ref_thread.lock().unwrap();
                *img = Some(Image::new(width, height));
            }

            while let Ok(Some(result)) = r.recv() {
                match result {
                    RenderEvent::RowsReady(unit_result) => {
                        d_println(format!("ImageBuilder: image fragment done, {} rows",
                                          unit_result.work_unit.row_end - unit_result.work_unit.row_start + 1));

                        let mut opt = img_ref_thread.lock().unwrap();
                        let img = opt.as_mut().unwrap();
                        for (i, row) in unit_result.rows.into_iter().enumerate() {
                            img.set_row(i + unit_result.work_unit.row_start, row);
                        }
                    },
                    RenderEvent::RenderingFinished { end_time, } => {
                        println!("rendering finished, total time {:?}", end_time.duration_since(start_time));
                        d_println(format!("ImageBuilder: rendering finished, total time {:?}",
                                          end_time.duration_since(start_time)));
                        let filename = scene_name.clone() + ".ppm";
                        let mut output_file = File::create(filename).unwrap();
                        let mut opt = img_ref_thread.lock().unwrap();
                        let img = opt.as_mut().unwrap();
                        img.write(&mut output_file);
                    },
                    _ => {
                        d_println(format!("ImageBuilder: got unexpected message"));
                        return;
                    },
                }
            }
        }).unwrap();

        Self {
            sender: s,
            thread_handle,
            image: img_ref,
        }
    }

    pub fn get_image(&self) -> Arc<Mutex<Option<Image>>> {
        self.image.clone()
    }

    pub fn sender(&self) -> Sender<Option<RenderEvent>> {
        self.sender.clone()
    }

    pub fn stop(self) {
        self.sender.send(None).ok();
        self.thread_handle.join().ok();
    }
}

struct CancellableIterator<T: Iterator> {
    items: T,
    cancelled: bool,
}

impl<T: Iterator> CancellableIterator<T> {
    pub fn new(items: T) -> Self {
        Self {
            items,
            cancelled: false,
        }
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

impl<T: Iterator> Iterator for CancellableIterator<T> {
    type Item = T::Item;

    fn next(&mut self) -> Option<T::Item> {
        if self.cancelled {
            None
        } else {
            self.items.next()
        }
    }
}
