
use crossbeam::channel::{Sender, Receiver, unbounded};
use crossbeam::sync::WaitGroup;
use crossbeam::SendError;
use std::fs::File;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

use serde_cbor::to_writer;
use serde_cbor::StreamDeserializer;
use serde_cbor::de::IoRead;
use std::net::TcpStream;

use crate::constants::DEFAULT_PORT;
use crate::scene::{Scene, SceneData};
use crate::color::Color;
use crate::image::Image;
use crate::job::{JobConfiguration, Job, JobID, JobIDAllocator, WorkUnit};
use crate::trace::Camera;

const DEBUG: bool = false;

fn d_println(s: String) {
    if DEBUG {
        let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!("{} {}", to_ms(t), s);
    }
}

fn to_ms(d: Duration) -> u64 {
    d.as_secs() * 1000 + d.subsec_nanos() as u64 / 1_000_000
}

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
    job_queue: Sender<Option<(Job, Sender<()>)>>,
    thread_handle: thread::JoinHandle<()>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum NetworkWorkerRequest {
    SetJob(Box<Job>),
    WorkUnit(WorkUnit),
    Done,
}

type WorkerRequest = Option<(Box<Job>, Receiver<Option<WorkUnit>>, Sender<Option<RenderEvent>>, WaitGroup)>;

pub struct WorkerHandle {
    sender: Sender<WorkerRequest>,
}

impl WorkerHandle {
    pub fn send(&self, j: Box<Job>, r: Receiver<Option<WorkUnit>>, s: Sender<Option<RenderEvent>>, wg: WaitGroup) -> Result<(), SendError<WorkerRequest>> {
        self.sender.send(Some((j, r, s, wg)))
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
        if workers.is_empty() {
            panic!("RenderManager::new: must provide at least one worker handle");
        }

        let (s, r): (Sender<Option<(Job, Sender<()>)>>, Receiver<Option<(Job, Sender<()>)>>) = unbounded();

        let handle = thread::Builder::new().name("RenderManager".to_string()).spawn(move || {
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

                let start_time = SystemTime::now();
                let start_event = RenderEvent::RenderingStarted { job_id: job.id, start_time, };
                result_sender.send(Some(start_event)).unwrap();

                workers.iter().for_each(|worker| {
                    ws.send(None).unwrap();
                    let job_boxed = Box::new(job.clone());
                    worker.send(job_boxed, wr.clone(), result_sender.clone(), wg.clone()).unwrap();
                });

                d_println(format!("Render manager: waiting for job completion"));

                wg.wait();

                d_println(format!("Render manager: job complete"));

                let end_time = SystemTime::now();
                result_sender.send(Some(RenderEvent::RenderingFinished { end_time, })).unwrap();

                match notify_done.send(()) {
                    Ok(_) => (),
                    Err(_) => (),
                }
            }

            d_println(format!("Render manager: shutting down"));
        }).unwrap();

        RenderManager {
            job_id_allocator: JobIDAllocator::new(),
            job_queue: s,
            thread_handle: handle,
        }
    }

    pub fn schedule_job(&mut self, scene_data: &SceneData, config: JobConfiguration) -> JobHandle {
        let id = self.job_id_allocator.next_id();
        let (s, r): (Sender<()>, Receiver<()>) = unbounded();
        let j = Job {
            scene_data: scene_data.clone(),
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

pub struct NetworkWorker {
    sender: Sender<WorkerRequest>,
    thread_handle: thread::JoinHandle<()>,
}

impl NetworkWorker {
    pub fn new(raw_endpoint: &String) -> NetworkWorker {
        let endpoint = match raw_endpoint.find(':') {
            None => format!("{}:{}", raw_endpoint, DEFAULT_PORT),
            Some(_) => raw_endpoint.clone(),
        };

        let tname = format!("NetworkWorker({})", endpoint);
        let stream = TcpStream::connect(endpoint.as_str()).unwrap();

        let (s, r): (Sender<WorkerRequest>, Receiver<WorkerRequest>) = unbounded();

        let handle = thread::Builder::new().name(tname).spawn(move || {
            let mut my_stream = stream;
            let stream_clone = my_stream.try_clone().unwrap();
            let mut stream_de: StreamDeserializer<'_, IoRead<TcpStream>, RenderEvent> =
                StreamDeserializer::new(IoRead::new(stream_clone));

            while let Ok(Some((job_boxed, recv_unit, send_result, wg))) = r.recv() {
                let job = *job_boxed;

                d_println(format!("Network worker: got job {:?}", job.id));

                to_writer(&mut my_stream, &NetworkWorkerRequest::SetJob(Box::new(job))).unwrap();

                let buf = 2;

                for _ in 0..buf {
                    let unit = recv_unit.recv().unwrap().unwrap();
                    to_writer(&mut my_stream, &NetworkWorkerRequest::WorkUnit(unit)).unwrap();
                }

                while let Ok(Some(unit)) = recv_unit.recv() {
                    d_println(format!("Network worker: got work unit {:?}", unit));

                    to_writer(&mut my_stream, &NetworkWorkerRequest::WorkUnit(unit)).unwrap();

                    match stream_de.next() {
                        None => {
                            d_println("Stream deserializer iterator finished".to_string());
                        },
                        Some(result) => {
                            match result {
                                Ok(ev) => {
                                    send_result.send(Some(ev)).unwrap();
                                },
                                Err(e) => {
                                    d_println(format!("Network worker got error from deserializer: {}", e));
                                    return;
                                }
                            }
                        }
                    }
                }

                for _ in 0..buf {
                    match stream_de.next() {
                        None => {
                            d_println("Stream deserializer iterator finished".to_string());
                        },
                        Some(result) => {
                            match result {
                                Ok(ev) => {
                                    send_result.send(Some(ev)).unwrap();
                                },
                                Err(e) => {
                                    d_println(format!("Network worker got error from deserializer: {}", e));
                                    return;
                                }
                            }
                        }
                    }
                }

                to_writer(&mut my_stream, &NetworkWorkerRequest::Done).unwrap();

                d_println(format!("Network worker finished job"));
                drop(wg);
            }

            d_println(format!("Network worker shutting down"));
        }).unwrap();

        NetworkWorker {
            sender: s,
            thread_handle: handle,
        }
    }
}

impl Worker for NetworkWorker {
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

pub struct LocalWorker {
    sender: Sender<WorkerRequest>,
    thread_handle: thread::JoinHandle<()>,
}

impl LocalWorker {
    pub fn new() -> LocalWorker {
        let (s, r): (Sender<WorkerRequest>, Receiver<WorkerRequest>) = unbounded();

        let handle = thread::Builder::new().name("LocalWorker".to_string()).spawn(move || {
            while let Ok(Some((job, recv_unit, send_result, wg))) = r.recv() {
                d_println(format!("Local worker: got job {:?}", job.id));

                let scene = Scene::from_data(job.scene_data, job.config);
                let camera = Camera::new(scene.camera_settings.clone(),
                                         scene.camera_basis.clone(),
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
                    match send_result.send(Some(ev)) {
                        Ok(()) => (),
                        Err(_) => {
                            return;
                        }
                    }
                }

                d_println(format!("Local worker finished job"));
                drop(wg);
            }

            d_println(format!("Local worker shutting down"));
        }).unwrap();

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

        let thread_handle = thread::Builder::new().name("ImageBuilder".to_string()).spawn(move || {
            let (scene_name, width, height) = match r.recv() {
                Ok(Some(RenderEvent::ImageInfo { scene_name, width, height } )) => (scene_name, width, height),
                _ => panic!("ImageBuilder: got unexpected message"),
            };

            d_println(format!("ImageBuilder: image {} x {} pixels", width, height));

            let start_time = match r.recv() {
                Ok(Some(RenderEvent::RenderingStarted { start_time, .. })) => start_time,
                _ => panic!("ImageBuilder: got unexpected message when expecting render start message"),
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
                    _ => panic!("ImageBuilder: got unexpected message"),
                }
            }
        }).unwrap();

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
