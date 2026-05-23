# bgpd

A minimal BGP daemon written in Rust. Supports BGP unicast and BGP Unnumbered.

> Work in progress — currently implements BGP OPEN message serialization and basic TCP session establishment.

## Features

- Run as a **server** (listens for incoming BGP connections) or **client** (initiates a session and sends a BGP OPEN message)
- BGP OPEN message construction (version 4, configurable ASN, hold time, BGP ID)
- CLI interface via [clap](https://github.com/clap-rs/clap)

## Usage

```
bgpd <MODE> <ADDRESS>
```

| Argument  | Description                          |
|-----------|--------------------------------------|
| `MODE`    | `server`, `client`, or `both`        |
| `ADDRESS` | Address and port, e.g. `0.0.0.0:8000` |

### Examples

```sh
# Start the server
bgpd server 0.0.0.0:8000

# Connect as a client and send a BGP OPEN message
bgpd client 0.0.0.0:8000
```

## Building & Running

```sh
# Build
make build

# Run as server
make server

# Run as client
make client
```

Default address is `0.0.0.0:8000` (configured in the `Makefile`).

## Requirements

- Rust (edition 2024)

## License

TBD
