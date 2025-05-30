use n0_future::time;
use tokio::sync::mpsc;

use crate::connector::{connector_delay, connector_throughput_limit};

use super::socket::Plug;

pub struct Wire {
    pub start: Plug,
    pub end: Plug,
}

impl Wire {
    pub fn new(capacity: usize) -> Self {
        // start -> end transmission
        let (start_sender, end_receiver) = mpsc::channel(capacity);
        // end -> start transmission
        let (end_sender, start_receiver) = mpsc::channel(capacity);
        let start = Plug {
            sender: start_sender,
            receiver: start_receiver,
        };
        let end = Plug {
            sender: end_sender,
            receiver: end_receiver,
        };

        Self { start, end }
    }

    pub fn new_delayed(capacity: usize, delay: time::Duration) -> Self {
        let wire_to_delay = Self::new(capacity);
        let wire_from_delay = Self::new(capacity);
        connector_delay(wire_to_delay.end, wire_from_delay.start, delay);
        Self {
            start: wire_to_delay.start,
            end: wire_from_delay.end,
        }
    }

    pub fn new_limited(capacity: usize, bytes_per_second: u32) -> Self {
        let wire_to_limiter = Self::new(capacity);
        let wire_from_limiter = Self::new(capacity);
        connector_throughput_limit(
            wire_to_limiter.end,
            wire_from_limiter.start,
            bytes_per_second,
        );
        Self {
            start: wire_to_limiter.start,
            end: wire_from_limiter.end,
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::{TestAddr, socket::VirtualSocket};

    use super::Wire;

    #[tokio::test]
    async fn test_wire_plugging() -> std::io::Result<()> {
        let wire = Wire::new(64);
        let socket0 = VirtualSocket::new(TestAddr(11), wire.start);
        let mut socket1 = VirtualSocket::new(TestAddr(99), wire.end);

        let contents = Bytes::copy_from_slice(b"Hello, world!");
        socket0
            .send_datagram(socket1.addr, contents.clone())
            .await?;

        let (source_addr, received) = socket1.receive_datagram().await?;

        assert_eq!(source_addr, socket0.addr);
        assert_eq!(received, contents);

        Ok(())
    }

    #[tokio::test]
    async fn test_wire_plugging_reverse() -> std::io::Result<()> {
        let wire = Wire::new(64);
        let socket0 = VirtualSocket::new(TestAddr(11), wire.end); // switch start and end
        let mut socket1 = VirtualSocket::new(TestAddr(99), wire.start);

        let contents = Bytes::copy_from_slice(b"Hello, world!");
        socket0
            .send_datagram(socket1.addr, contents.clone())
            .await?;

        let (source_addr, received) = socket1.receive_datagram().await?;

        assert_eq!(source_addr, socket0.addr);
        assert_eq!(received, contents);

        Ok(())
    }
}
