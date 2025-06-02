use bytes::Bytes;
use n0_future::time;
use testresult::TestResult;

use virtual_sockets::{
    TestAddr, endpoint::TestEndpoint, socket::VirtualSocket, switch::Switch, wire::Wire,
};

#[tokio::test]
async fn test_quinn_connection_over_switch() -> TestResult {
    // Virtual sockets setup
    let switch = Switch::new();
    let server_socket = switch.connect_socket(TestAddr(42)).await;
    let client_socket = switch.connect_socket(TestAddr(111)).await;
    let server_addr = server_socket.addr;

    let server_ep = TestEndpoint::server(server_socket);
    let mut client_ep = TestEndpoint::client(client_socket);
    client_ep.make_client_for(&server_ep);

    // Run server task
    tokio::spawn({
        let server_ep = server_ep.clone();
        async move {
            // Simple echo loop
            while let Some(incoming) = server_ep.accept().await {
                let conn = incoming.accept()?.await?;
                conn.close(0u32.into(), b"bye!");
            }

            TestResult::Ok(())
        }
    });

    let conn = client_ep.connect(server_addr, "localhost")?.await?;

    conn.closed().await;

    client_ep.close(1u32.into(), b"endpoint closed");
    server_ep.close(1u32.into(), b"endpoint closed");

    client_ep.wait_idle().await;
    server_ep.wait_idle().await;
    Ok(())
}

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
            // accept exactly one incoming connection
            let conn = server_ep.accept().await.unwrap().await?;
            let mut stream = conn.accept_uni().await?;

            // read to completion
            while let Some(_chunk) = stream.read_chunk(10_000, true).await? {}

            conn.close(0u32.into(), b"bye!");
            TestResult::Ok(())
        }
    });

    let conn = client_ep
        .connect_with(server_ep.client_config(), server_addr, "localhost")?
        .await?;

    let mut stream = conn.open_uni().await?;
    let buf = Bytes::copy_from_slice(&b"Hello, world!".repeat(1000));
    const NUM: usize = 1_000;
    for _ in 0..NUM {
        stream.write_chunk(buf.clone()).await?;
    }
    stream.finish()?;

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
