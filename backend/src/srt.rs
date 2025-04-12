use std::{net::SocketAddr, sync::Arc};

use futures::{StreamExt, TryStreamExt};
use srt_tokio::{
    SrtListener, SrtSocket,
    access::{RejectReason, ServerRejectReason},
    options::{SocketAddress, StreamId},
};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::state::State;

pub async fn srt_listen(server_bind: impl TryInto<SocketAddress>, state: Arc<Mutex<State>>) {
    info!("Starting SRT server...");
    let (_server, mut incoming) = SrtListener::builder()
        .bind(server_bind)
        .await
        .expect("Failed to bind to address and port");

    while let Some(request) = incoming.incoming().next().await {
        let addr = request.remote();
        let stream_id = request.stream_id().cloned();

        if stream_id.is_none() {
            error!(
                "Rejecting connection from {} because no stream ID was provided",
                addr
            );
            request
                .reject(RejectReason::Server(ServerRejectReason::Unauthorized))
                .await
                .unwrap();
            continue;
        }
        let stream_id = stream_id.unwrap();
        info!(
            "Incoming connection from {} with stream ID: {}",
            addr, stream_id
        );

        {
            let state = state.clone();
            tokio::spawn(async move {
                {
                    let mut state = state.lock().await;

                    if state.add_stream(stream_id.to_string()).is_err() {
                        error!(
                            "[{} - {}] Stream ID already exists, rejecting connection.",
                            addr, stream_id
                        );
                        request
                            .reject(RejectReason::Server(ServerRejectReason::Unauthorized))
                            .await
                            .unwrap();
                        return;
                    }
                }

                match request.accept(None).await {
                    Ok(socket) => {
                        info!(
                            "[{} - {}] SRT connection fully accepted for stream.",
                            addr, stream_id
                        );
                        handle_stream(state, socket, addr, stream_id).await;
                    }
                    Err(e) => {
                        error!(
                            "[{} - {}] Failed to fully accept stream: {:?}",
                            addr, stream_id, e
                        );
                        let mut state = state.lock().await;
                        state.remove_stream(&stream_id.to_string()).unwrap();
                    }
                }
            });
        }
    }
}

async fn handle_stream(
    state: Arc<Mutex<State>>,
    mut socket: SrtSocket,
    addr: SocketAddr,
    stream_id: StreamId,
) {
    let sender = {
        let mut state = state.lock().await;
        state.get_stream_sender(&stream_id.to_string()).unwrap()
    };

    loop {
        match socket.try_next().await {
            Ok(Some((_timestamp, data))) => {
                let _ = sender.send(data);
            }
            Ok(None) => {
                info!("[{} - {}] Stream closed.", addr, stream_id);
                let mut state = state.lock().await;
                state.remove_stream(&stream_id.to_string()).unwrap();
                break;
            }
            Err(e) => {
                error!(
                    "[{} - {}] Error receiving data, closing stream: {}",
                    addr, stream_id, e
                );
                let mut state = state.lock().await;
                state.remove_stream(&stream_id.to_string()).unwrap();
                break;
            }
        }
    }
}
