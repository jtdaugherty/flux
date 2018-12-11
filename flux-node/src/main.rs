
extern crate fluxcore;
extern crate serde_cbor;
extern crate crossbeam;

use crossbeam::channel::{Sender, Receiver, unbounded};
use crossbeam::sync::WaitGroup;

use fluxcore::job::*;
use fluxcore::manager::{Worker, LocalWorker, NetworkWorkerRequest, RenderEvent, WorkerHandle};

use serde_cbor::StreamDeserializer;
use serde_cbor::de::IoRead;
use serde_cbor::to_writer;

use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io;

fn handle_client(stream: TcpStream, worker: &WorkerHandle) -> io::Result<()> {
    let peer = stream.peer_addr()?;

    println!("Got connection from {}", peer);

    let thread_stream = stream.try_clone().unwrap();
    let stream_de: StreamDeserializer<'_, IoRead<TcpStream>, NetworkWorkerRequest> =
        StreamDeserializer::new(IoRead::new(stream));

    let (wu_send, wu_recv): (Sender<Option<WorkUnit>>, Receiver<Option<WorkUnit>>) = unbounded();
    let (re_send, re_recv): (Sender<Option<RenderEvent>>, Receiver<Option<RenderEvent>>) = unbounded();
    let wg = WaitGroup::new();

    let t_handle = thread::spawn(move || {
        let mut my_stream = thread_stream;
        while let Ok(Some(ev)) = re_recv.recv() {
            to_writer(&mut my_stream, &ev).unwrap();
        }
    });

    for result in stream_de.into_iter() {
        match result {
            Ok(req) => {
                match req {
                    NetworkWorkerRequest::SetJob(j) => {
                        println!("Got job");
                        let send_result = worker.send(j, wu_recv.clone(), re_send.clone(), wg.clone());
                        match send_result {
                            Ok(()) => {
                                println!("Sent job to local worker");
                            },
                            Err(e) => {
                                println!("Could not send job to local worker: {}", e);
                            }
                        }
                    },
                    NetworkWorkerRequest::WorkUnit(u) => {
                        println!("Got work unit, sending to worker");
                        wu_send.send(Some(u)).unwrap();
                    },
                    NetworkWorkerRequest::Done => {
                        wu_send.send(None).unwrap();
                    }
                }
            },
            Err(err) => {
                println!("Got error: {}", err);
            }
        }
    }

    t_handle.join().unwrap();

    Ok(())
}

fn run_server(bind_address: String, worker: LocalWorker) -> io::Result<()> {
    let listener = TcpListener::bind(bind_address)?;
    let handle = worker.handle();

    for stream in listener.incoming() {
        handle_client(stream?, &handle)?;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let listen_host = "0.0.0.0";
    let listen_port = "2000";
    let bind_address = format!("{}:{}", listen_host, listen_port);

    println!("Bind address: {}", bind_address);

    let worker = LocalWorker::new();
    run_server(bind_address, worker)?;

    Ok(())
}
