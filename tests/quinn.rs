use virtual_sockets::{TestAddr, endpoint::TestEndpoint, switch::Switch};

#[tokio::test]
async fn test_connect_with_virtual_socket() {
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
                let conn = incoming.accept().unwrap().await.unwrap();
                conn.close(0u32.into(), b"bye!");
            }
        }
    });

    // server_socket.set_paused(true);
    // let mut wiretap = server_socket.wiretap();

    // let (conn, to_resend) = tokio::join!(
    //     async {
    // Connect from the client
    let conn = client_ep
        .connect(server_addr, "localhost")
        .unwrap()
        .await
        .unwrap();
    //         conn
    //     },
    //     async {
    //         // Grab the last
    //         let mut to_resend = wiretap.recv().await.unwrap();
    //         to_resend.src_ip = SocketAddr::new([192, 168, 0, 133].into(), 1234);
    //         server_socket
    //             .try_send(&to_resend.as_quinn_transmit())
    //             .expect("couldn't resend packet");
    //         server_socket.set_paused(false);
    //         to_resend
    //     }
    // );

    conn.closed().await;

    client_ep.close(1u32.into(), b"endpoint closed");
    server_ep.close(1u32.into(), b"endpoint closed");

    // client_socket
    //     .try_send(&to_resend.as_quinn_transmit())
    //     .expect("couldn't resend packet");

    client_ep.wait_idle().await;
    server_ep.wait_idle().await;
}
