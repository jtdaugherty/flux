
use crossbeam::channel::{Sender, Receiver, unbounded};
use crossbeam::sync::WaitGroup;
use std::fs::File;
use std::thread;
use std::sync::{Arc, Mutex};

use scene::{Scene, SceneData};
use color::Color;
use image::Image;
use job::{JobConfiguration, Job, JobIDAllocator, WorkUnit};
use trace::Camera;

const DEBUG: bool = false;

use std::time::{SystemTime, UNIX_EPOCH, Duration};

fn d_println(s: String) {
    if DEBUG {
        let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!("{} {}", to_ms(t), s);
    }
}

fn to_ms(d: Duration) -> u64 {
    d.as_secs() * 1000 + d.subsec_nanos() as u64 / 1_000_000
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

type WorkerRequest = Option<(Job, Receiver<Option<WorkUnit>>, Sender<Option<RenderEvent>>, WaitGroup)>;

pub struct WorkerHandle {
    sender: Sender<WorkerRequest>,
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
    sender: Sender<WorkerRequest>,
    thread_handle: thread::JoinHandle<()>,
}

impl LocalWorker {
    pub fn new() -> LocalWorker {
        let (s, r): (Sender<WorkerRequest>, Receiver<WorkerRequest>) = unbounded();

        let handle = thread::spawn(move || {
            while let Ok(Some((job, recv_unit, send_result, wg))) = r.recv() {
                d_println(format!("Local worker: got job {:?}", job.id));

                let scene = Scene::from_data(job.scene_data);
                let camera = Camera::new(scene.camera_settings.clone(),
                                         job.config,
                                         scene.output_settings.image_width,
                                         scene.camera_data.zoom_factor,
                                         scene.camera_data.view_plane_distance,
                                         scene.camera_data.focal_distance,
                                         scene.camera_data.lens_radius);

                while let Ok(Some(unit)) = recv_unit.recv() {
                    d_println(format!("Local worker: got work unit {:?}", unit));

                    d_println(format!("Starting render"));
                    let r = camera.render(&scene, unit);
                    d_println(format!("render done"));

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
    image: Arc<Mutex<Option<Image>>>,
}

impl ImageBuilder {
    pub fn new() -> ImageBuilder {
        let (s, r): (Sender<Option<RenderEvent>>, Receiver<Option<RenderEvent>>) = unbounded();
        let img_ref = Arc::new(Mutex::new(None));
        let img_ref_thread = img_ref.clone();

        let thread_handle = thread::spawn(move || {
            let (scene_name, width, height) = match r.recv() {
                Ok(Some(RenderEvent::ImageInfo { scene_name, width, height } )) => (scene_name, width, height),
                _ => panic!("ImageBuilder: got unexpected message"),
            };

            d_println(format!("ImageBuilder: image {} x {} pixels", width, height));

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
                        let mut img = opt.as_mut().unwrap();
                        for (i, row) in unit_result.rows.into_iter().enumerate() {
                            img.set_row(i + unit_result.work_unit.row_start, row);
                        }
                    },
                    RenderEvent::RenderingFinished => {
                        d_println(format!("ImageBuilder: rendering finished"));
                        let filename = scene_name.clone() + ".ppm";
                        let mut output_file = File::create(filename).unwrap();
                        let mut opt = img_ref_thread.lock().unwrap();
                        let mut img = opt.as_mut().unwrap();
                        img.write(&mut output_file);
                    },
                    _ => panic!("ImageBuilder: got unexpected message"),
                }
            }
        });

        ImageBuilder {
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
        self.sender.send(None).unwrap();
        self.thread_handle.join().unwrap();
    }
}
