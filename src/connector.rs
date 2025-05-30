use n0_future::time;
use tokio::sync::mpsc;

use crate::{OwnedTransmit, socket::Plug};

pub fn connector_delay(start: Plug, end: Plug, delay: time::Duration) {
    let forward_receiver = start.receiver;
    let forward_sender = end.sender;
    let backward_receiver = end.receiver;
    let backward_sender = start.sender;
    tokio::spawn(connect_delay_oneway(
        forward_receiver,
        forward_sender,
        delay,
    ));
    tokio::spawn(connect_delay_oneway(
        backward_receiver,
        backward_sender,
        delay,
    ));
}

pub fn connector_throughput_limit(start: Plug, end: Plug, bytes_per_second: u32) {
    let time_per_byte = time::Duration::from_secs(1) / bytes_per_second;
    let forward_receiver = start.receiver;
    let forward_sender = end.sender;
    let backward_receiver = end.receiver;
    let backward_sender = start.sender;
    tokio::spawn(connect_throughput_limit_oneway(
        forward_receiver,
        forward_sender,
        time_per_byte,
    ));
    tokio::spawn(connect_throughput_limit_oneway(
        backward_receiver,
        backward_sender,
        time_per_byte,
    ));
}

async fn connect_delay_oneway(
    mut receive: mpsc::Receiver<OwnedTransmit>,
    send: mpsc::Sender<OwnedTransmit>,
    delay: time::Duration,
) {
    while let Some(transmit) = receive.recv().await {
        time::sleep(delay).await;
        if send.send(transmit).await.is_err() {
            return; // broken wire
        }
    }
}

async fn connect_throughput_limit_oneway(
    mut receive: mpsc::Receiver<OwnedTransmit>,
    send: mpsc::Sender<OwnedTransmit>,
    time_per_byte: time::Duration,
) {
    while let Some(transmit) = receive.recv().await {
        let delay = time_per_byte * transmit.contents.len() as u32;
        time::sleep(delay).await;
        if send.send(transmit).await.is_err() {
            return; // broken wire
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use n0_future::time;
    use testresult::TestResult;

    use crate::{TestAddr, endpoint::TestEndpoint, socket::VirtualSocket, wire::Wire};

    async fn run_quinn_test_rtt(wire: Wire) -> TestResult<time::Duration> {
        let client_socket = VirtualSocket::new(TestAddr(42), wire.start);
        let server_socket = VirtualSocket::new(TestAddr(111), wire.end);

        let server_addr = server_socket.addr;
        let server_ep = TestEndpoint::server(server_socket);
        let client_ep = TestEndpoint::client(client_socket);

        // Run server task
        tokio::spawn({
            let server_ep = server_ep.clone();
            async move {
                let conn = server_ep.accept().await.unwrap().await.unwrap();
                let mut stream = conn.accept_uni().await.unwrap();

                // read to completion
                while let Some(_chunk) = stream.read_chunk(10_000, true).await.unwrap() {}

                conn.close(0u32.into(), b"bye!");
            }
        });

        let conn = client_ep
            .connect_with(server_ep.client_config(), server_addr, "localhost")
            .unwrap()
            .await
            .unwrap();

        let mut stream = conn.open_uni().await.unwrap();
        let buf = Bytes::copy_from_slice(&b"Hello, world!".repeat(1000));
        const NUM: usize = 1_000;
        for _ in 0..NUM {
            stream.write_chunk(buf.clone()).await.unwrap();
        }
        stream.finish().unwrap();

        conn.closed().await;

        let rtt = conn.rtt();

        client_ep.close(1u32.into(), b"endpoint closed");
        server_ep.close(1u32.into(), b"endpoint closed");
        client_ep.wait_idle().await;
        server_ep.wait_idle().await;

        Ok(rtt)
    }

    #[tokio::test]
    async fn test_quinn_rtt_baseline() -> TestResult {
        // tokio::time::pause(); // actually run unpaused to make sure even in that case it's fast
        let wire = Wire::new(1);
        let rtt = run_quinn_test_rtt(wire).await?;
        println!("{rtt:?}");
        assert!(rtt < time::Duration::from_millis(10));
        Ok(())
    }

    #[tokio::test]
    async fn test_delayed_quinn_rtt() -> TestResult {
        tokio::time::pause();
        let wire = Wire::new_delayed(1, time::Duration::from_millis(50));
        let rtt = run_quinn_test_rtt(wire).await?;
        println!("{rtt:?}");
        assert!(rtt > time::Duration::from_millis(100));
        Ok(())
    }

    #[tokio::test]
    async fn test_limited_quinn_rtt() -> TestResult {
        tokio::time::pause();
        let wire = Wire::new_limited(1, 10000);
        let rtt = run_quinn_test_rtt(wire).await?;
        println!("10kbps: {rtt:?}");
        assert!(rtt > time::Duration::from_millis(100));
        Ok(())
    }
}
