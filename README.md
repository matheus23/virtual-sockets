# virtual-sockets

This crate allows you to simulate virtual internets (effectively UDP only at the moment), utilizing tokio's tasks, mpsc channels and `tokio::time`.
This allows your simulations to use `tokio::time::pause()` to simulate time/throughput without actually waiting for any real clocks to advance.

The `VirtualSocket` type implements `quinn::AsyncUdpSocket`, so it can be used as a backend for `quinn` to simulate QUIC traffic (this crate's first intended use case).

## Examples

### Wires

Create two sockets that can talk to each other via a simple "wire" (IP addresses won't be used to determine where packets end up):

```rs
use virtual_sockets::{wire::Wire, socket::VirtualSocket, TestAddr};
use bytes::Bytes;
use tokio::time::Duration;

let wire = Wire::new(1); // 1 is the internal buffer size
let socket0 = VirtualSocket::new(TestAddr(11), wire.start);
let socket1 = VirtualSocket::new(TestAddr(99), wire.end);

let datagram = Bytes::from_static(b"Hello, world!");
socket0.send_datagram(TestAddr(99), datagram).await?;
let (src, received) = socket1.receive_datagram().await?;
assert_eq!(received, datagram);
```

Wires come in different sorts.
- They can include delays: `Wire::new_delayed(1, Duration::from_millis(50))` (50ms one-way latency in both directions)
- They can be throughput-limited: `Wire::new_limited(1, 32_000)` (32 kilobytes per second)

These wires are very simple and don't model the way the real works effectively (e.g. the limited wires will never drop packets), but it's possible to improve these.

### Switches

It's also possible to create simple switches.
These switches also allow you to very easily wire up new sockets or connect & disconnect sockets at runtime:

```rs
use virtual_sockets::{switch::Switch, TestAddr};

let switch = Switch::new();
let socket0 = switch.connect_socket(TestAddr(42)).await?;
let socket1 = switch.connect_socket(TestAddr(111)).await?;

// use sockets like above
```

## Future

In the future it'd be nice to simulate networking components more real-world-like:
- store internal queues of packets to transmit, tail-drop packets
- implement active queue management (ACM) and explicit congestion notifications (ECN)

Additionally, various additional network components could be implemented:
- routers with network address translation (NAT) (in both endpoint-dependent and endpoint-independent mappings)
- stateful firewalls
- a component that simulates a device having multiple network interfaces at once and simulates the OS choosing the "optimal" default interface to send on

