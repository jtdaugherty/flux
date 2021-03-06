
use crossbeam::channel::{Sender, Receiver, unbounded};
use std::thread;
use std::net::TcpStream;
use std::io;

use rayon;
use serde_cbor::to_writer;
use serde_cbor::StreamDeserializer;
use serde_cbor::de::IoRead;

use crate::constants::DEFAULT_PORT;
use crate::scene::{Scene};
use crate::trace::Camera;
use crate::manager::*;
use crate::job::{Job, WorkUnit};
use crate::debug::d_println;

pub struct LocalWorker {
    sender: Sender<WorkerRequest>,
    thread_handle: thread::JoinHandle<()>,
    worker_info: WorkerInfo,
}

impl LocalWorker {
    pub fn new(num_threads: usize) -> Self {
        let tp_result = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global();
        match tp_result {
            Ok(_) => {
                d_println(format!("LocalWorker set global thread pool size to {}", num_threads));
            },
            Err(_) => {
                println!("Warning: global thread pool already configured, number of threads is {}",
                         rayon::current_num_threads());
            }
        }

        let (s, r): (Sender<WorkerRequest>, Receiver<WorkerRequest>) = unbounded();

        let handle = thread::Builder::new().name("LocalWorker".to_string()).spawn(move || {
            'main: while let Ok(Some((job, recv_unit, send_result, wg))) = r.recv() {
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

                while let Ok(unit) = recv_unit.recv() {
                    d_println(format!("Local worker: got work unit {:?}", unit));

                    d_println(format!("Starting render"));
                    let r = camera.render(&scene, unit);
                    d_println(format!("render done"));

                    let ev = RenderEvent::RowsReady(r);
                    match send_result.send(Some(ev)) {
                        Ok(()) => (),
                        Err(_) => {
                            d_println(format!("LocalWorker advancing to next job to due result send error"));
                            continue 'main;
                        }
                    }
                }

                d_println(format!("Local worker finished job"));
                drop(wg);
            }

            d_println(format!("Local worker shutting down"));
        }).unwrap();

        Self {
            sender: s,
            thread_handle: handle,
            worker_info: WorkerInfo {
                num_threads,
            },
        }
    }
}

impl Worker for LocalWorker {
    fn handle(&self) -> WorkerHandle {
        WorkerHandle::new(self.sender.clone())
    }

    fn stop(self) {
        self.sender.send(None).ok();
        self.thread_handle.join().ok();
    }

    fn info(&self) -> WorkerInfo {
        self.worker_info
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum NetworkWorkerRequest {
    SetJob(Box<Job>),
    WorkUnit(WorkUnit),
    Done,
}

pub struct NetworkWorker {
    sender: Sender<WorkerRequest>,
    thread_handle: thread::JoinHandle<()>,
    worker_info: WorkerInfo,
}

impl NetworkWorker {
    pub fn new(raw_endpoint: &String) -> Result<Self, io::Error> {
        let endpoint = match raw_endpoint.find(':') {
            None => format!("{}:{}", raw_endpoint, DEFAULT_PORT),
            Some(_) => raw_endpoint.clone(),
        };

        let tname = format!("NetworkWorker({})", endpoint);
        match TcpStream::connect(endpoint.as_str()) {
            Err(e) => Err(e),
            Ok(st) => {
                let stream_clone = st.try_clone().unwrap();
                let mut stream_info_de: StreamDeserializer<'_, IoRead<TcpStream>, WorkerInfo> =
                    StreamDeserializer::new(IoRead::new(stream_clone));

                println!("Getting info");
                // Expect that the first thing to do is read a usize
                // from the network stream indicating the number of
                // threads that the remote end will be using.
                let worker_info: WorkerInfo = match stream_info_de.next() {
                    Some(Ok(i)) => i,
                    _ => {
                        panic!("Could not get info from network node")
                    },
                };

                println!("Got info");

                let (s, r): (Sender<WorkerRequest>, Receiver<WorkerRequest>) = unbounded();

                let handle = thread::Builder::new().name(tname).spawn(move || {
                    let mut my_stream = st;
                    let stream_clone = my_stream.try_clone().unwrap();
                    let mut stream_de: StreamDeserializer<'_, IoRead<TcpStream>, RenderEvent> =
                        StreamDeserializer::new(IoRead::new(stream_clone));

                    while let Ok(Some((job_boxed, recv_unit, send_result, wg))) = r.recv() {
                        let job = *job_boxed;

                        d_println(format!("Network worker: got job {:?}", job.id));

                        to_writer(&mut my_stream, &NetworkWorkerRequest::SetJob(Box::new(job))).unwrap();

                        let buf = 2;
                        let mut sent = 0;

                        for _ in 0..buf {
                            match recv_unit.recv() {
                                Err(e) => {
                                    d_println(format!("Error sending initial work unit: {}", e));
                                }
                                Ok(unit) => {
                                    d_println(format!("Sending initial work unit"));
                                    to_writer(&mut my_stream, &NetworkWorkerRequest::WorkUnit(unit)).unwrap();
                                    sent += 1;
                                },
                            };
                        }

                        d_println(format!("NetworkWorker sending remaining work units"));

                        while let Ok(unit) = recv_unit.recv() {
                            d_println(format!("Network worker: got work unit {:?}", unit));

                            to_writer(&mut my_stream, &NetworkWorkerRequest::WorkUnit(unit)).unwrap();

                            match stream_de.next() {
                                None => {
                                    d_println("Stream deserializer iterator finished".to_string());
                                },
                                Some(result) => {
                                    match result {
                                        Ok(ev) => {
                                            d_println(format!("Network worker got a render event from the remote end"));
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

                        d_println(format!("NetworkWorker collecting final {} results", sent));

                        for _ in 0..sent {
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

                        d_println(format!("NetworkWorker sending Done message"));

                        to_writer(&mut my_stream, &NetworkWorkerRequest::Done).unwrap();

                        d_println(format!("Network worker finished job"));
                        drop(wg);
                    }

                    d_println(format!("Network worker shutting down"));
                }).unwrap();

                Ok(Self {
                    sender: s,
                    thread_handle: handle,
                    worker_info,
                })
            }
        }
    }
}

impl Worker for NetworkWorker {
    fn handle(&self) -> WorkerHandle {
        WorkerHandle::new(self.sender.clone())
    }

    fn stop(self) {
        self.sender.send(None).ok();
        self.thread_handle.join().ok();
    }

    fn info(&self) -> WorkerInfo {
        self.worker_info
    }
}
