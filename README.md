# bgpd

A minimal BGP daemon written in Rust.

> Work in progress — currently supports BGP session establishment, KEEPALIVE exchange, hold timers, NOTIFICATION
> messages, and configuration-driven neighbors.

## Features

### Session Management

* BGP OPEN message serialization and parsing
* BGP KEEPALIVE message support
* BGP NOTIFICATION message support
* TCP session establishment and teardown
* Hold timer monitoring
* Periodic KEEPALIVE transmission
* Event-driven session architecture
* Basic BGP Finite State Machine (FSM)

### Router Daemon

* Configuration-driven router startup
* Listener for inbound BGP connections
* Outbound neighbor session initiation
* Passive neighbors (listen-only)
* Multiple concurrent BGP sessions

## Configuration

Routers are configured using TOML.

Example:

```toml
router_id = "127.0.0.1"
local_as = 65001
listen_addr = "127.0.0.1:8000"

[[neighbors]]
address = "127.0.0.1:9000"
peer_as = 65002
passive = true
```

### Router Fields

| Field         | Description                    |
|---------------|--------------------------------|
| `router_id`   | BGP Router ID                  |
| `local_as`    | Local Autonomous System Number |
| `listen_addr` | Address and port to listen on  |

### Neighbor Fields

| Field     | Description                                    |
|-----------|------------------------------------------------|
| `address` | Neighbor address and port                      |
| `peer_as` | Expected remote ASN                            |
| `passive` | If `true`, do not initiate outbound connection |

## Usage

```sh
bgpd <CONFIG_PATH>
```

### Examples

```sh
bgpd examples/configs/server.toml

bgpd examples/configs/client.toml
```

## Building & Running

```sh
# Build
make build

# Run example router configurations
make server
make client
```

## Current Protocol Support

| Message Type | Status |
|--------------|--------|
| OPEN         | ✔️     |
| KEEPALIVE    | ✔️     |
| NOTIFICATION | ✔️     |
| UPDATE       | ✔️     |

## Current Limitations

* No UPDATE message support yet
* No route advertisement or withdrawal
* No RIB/FIB implementation
* No route selection logic
* No BGP capabilities negotiation
* No session collision detection
* No IPv6 support yet

## Roadmap

* UPDATE message support
* Route advertisements and withdrawals
* Routing Information Base (RIB)
* Configuration reload support
* BGP Unnumbered support
* IPv6 address families
* Session registry and management
* Event-driven timer scheduler

## Requirements

* Rust (edition 2024)

## License

TBD

