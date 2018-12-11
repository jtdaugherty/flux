
extern crate fluxcore;

use fluxcore::job::*;

use std::net::{TcpListener, TcpStream};
use std::io;
use std::io::Read;
use std::io::Write;

fn handle_client(mut stream: TcpStream) -> io::Result<()> {
    let peer = stream.peer_addr()?;

    println!("Got connection from {}", peer);

    let mut buf: [u8; 128] = [0; 128];

    match stream.read(&mut buf) {
        Ok(num) => {
            println!("Read {} bytes: \"{}\"", num, String::from_utf8_lossy(&buf[0..num]));
            stream.write(&buf);
        }
        _ => {
            println!("Failed to read from client");
        }
    }

    Ok(())
}

fn run_server(bind_address: String) -> io::Result<()> {
    let listener = TcpListener::bind(bind_address)?;

    for stream in listener.incoming() {
        handle_client(stream?);
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let listen_host = "127.0.0.1";
    let listen_port = "2000";
    let bind_address = format!("{}:{}", listen_host, listen_port);

    println!("Bind address: {}", bind_address);

    run_server(bind_address);

    Ok(())
}
