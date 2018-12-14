
extern crate fluxcore;
extern crate serde_cbor;
extern crate crossbeam;
extern crate clap;
extern crate num_cpus;

use crossbeam::channel::{Sender, Receiver, unbounded};
use crossbeam::sync::WaitGroup;

use fluxcore::job::*;
use fluxcore::constants::DEFAULT_PORT;
use fluxcore::manager::{Worker, LocalWorker, NetworkWorkerRequest, RenderEvent, WorkerHandle};

use serde_cbor::StreamDeserializer;
use serde_cbor::de::IoRead;
use serde_cbor::to_writer;

use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io;
use std::str::FromStr;

use clap::{Arg, App};

fn handle_client(stream: TcpStream, worker: &WorkerHandle) -> io::Result<()> {
    let peer = stream.peer_addr()?;

    println!("Got connection from {}", peer);

    let thread_stream = stream.try_clone().unwrap();
    let stream_de: StreamDeserializer<'_, IoRead<TcpStream>, NetworkWorkerRequest> =
        StreamDeserializer::new(IoRead::new(stream));

    let (wu_send, wu_recv): (Sender<WorkUnit>, Receiver<WorkUnit>) = unbounded();
    let (re_send, re_recv): (Sender<Option<RenderEvent>>, Receiver<Option<RenderEvent>>) = unbounded();
    let wg = WaitGroup::new();

    let t_handle = thread::spawn(move || {
        let mut my_stream = thread_stream;
        println!("Work unit result thread started");
        while let Ok(Some(ev)) = re_recv.recv() {
            println!("Got results from local worker, sending to manager");
            match to_writer(&mut my_stream, &ev) {
                Ok(()) => (),
                Err(e) => {
                    println!("Manager connection error: {}", e);
                    return;
                }
            }
        }
        println!("Work unit result thread stopping");
    });

    for result in stream_de {
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
                        wu_send.send(u).unwrap();
                    },
                    NetworkWorkerRequest::Done => {
                        println!("Got done message, shutting down");
                        return Ok(())
                    }
                }
            },
            Err(err) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("{}", err)));
            }
        }
    }

    drop(re_send);

    t_handle.join().unwrap();

    Ok(())
}

fn run_server(bind_address: String, worker: &LocalWorker) -> io::Result<()> {
    let listener = TcpListener::bind(bind_address)?;
    let handle = worker.handle();

    for stream in listener.incoming() {
        match handle_client(stream?, &handle) {
            Ok(()) => {
            },
            Err(e) => {
                println!("run_server: handle_client exited with {}", e);
            }
        }
    }

    Ok(())
}

struct Config {
    pub listen_host: String,
    pub listen_port: String,
    pub num_threads: usize,
}

fn config_from_args() -> Config {
    let app = App::new("flux-node")
        .author("Jonathan Daugherty <cygnus@foobox.com>")
        .about("Network rendering server for the flux ray tracer")
        .arg(Arg::with_name("host")
             .short("h")
             .long("host")
             .value_name("ADDRESS")
             .help("Listen for requests on this address")
             .takes_value(true))
        .arg(Arg::with_name("port")
             .short("p")
             .long("port")
             .help("Listen on this TCP port")
             .takes_value(true))
        .arg(Arg::with_name("threads")
             .short("t")
             .long("threads")
             .help("Number of rendering threads (defaults to number of logical CPUs)")
             .takes_value(true));

    let ms = app.get_matches();
    let default_host = "0.0.0.0";
    let default_port = DEFAULT_PORT;

    Config {
        listen_host: ms.value_of("host").unwrap_or(default_host).to_string(),
        listen_port: ms.value_of("port").unwrap_or(default_port).to_string(),
        num_threads: match ms.value_of("threads") {
            None => num_cpus::get(),
            Some(t) => usize::from_str(t).unwrap(),
        },
    }
}

fn main() -> io::Result<()> {
    let config = config_from_args();
    let bind_address = format!("{}:{}", config.listen_host, config.listen_port);

    println!("Bind address: {}", bind_address);

    let worker = LocalWorker::new(config.num_threads);
    run_server(bind_address, &worker)?;

    worker.stop();

    Ok(())
}
