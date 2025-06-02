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
