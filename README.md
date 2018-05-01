
# rrat

An rust CLI app that connects to a server then opens a shell and redirects stdin/stdout through the network socket.
Can be used in conjunction with the metasploit framework reverse_tcp shell listener.


## Building

Ensure rust is installed (https://www.rust-lang.org/) then run:

```bash
$ cargo build
```

The host and port are hard-coded towards the top of main.rs so you'll want to change these before building.

## Usage / running

Output is written using the `env_logger` crate so by default you won't see anything unless you set the "RUST_LOG" environment variable.

To see output while running use:

```bash
$ RUST_LOG=rrat cargo run
```

or if running an already compiled executable:

```bash
$ RUST_LOG=rrat ./rrat
```

Example output after running `rrat` then starting a meterpreter listener.

[![asciicast](https://asciinema.org/a/WEoEK34wVe8iKmS5BIpxWOl8Y.png)](https://asciinema.org/a/WEoEK34wVe8iKmS5BIpxWOl8Y)

