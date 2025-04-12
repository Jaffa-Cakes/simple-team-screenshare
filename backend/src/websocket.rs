use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::{
    Router,
    extract::{
        Path, State as AxumState, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::get,
};
use futures::{SinkExt, StreamExt};
use include_dir::{Dir, include_dir};
use tokio::{
    net::TcpListener,
    select,
    sync::{Mutex, broadcast::error::RecvError},
    time::interval,
};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::state::State;

static WEB_APP_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../frontend/dist");

pub async fn websocket_listen(server_bind: SocketAddr, state: Arc<Mutex<State>>) {
    info!("Starting HTTP and WebSocket server...");
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/streams", get(incoming_stream_list_handler))
        .route("/streams/{stream_id}", get(incoming_stream_handler))
        .route("/{*path}", get(serve_embedded_file))
        .with_state(state);

    let listener = TcpListener::bind(server_bind)
        .await
        .expect("Failed to bind to address and port");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("Failed to start server");
}

async fn serve_index() -> impl IntoResponse {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html")
        .body(Body::from(
            WEB_APP_DIR
                .get_file("index.html")
                .unwrap()
                .contents()
                .to_vec(),
        ))
        .unwrap()
}

async fn serve_embedded_file(path: Path<String>) -> impl IntoResponse {
    let path = path.0.trim_start_matches('/');
    let file = WEB_APP_DIR.get_file(path);

    match file {
        Some(file) => {
            let mime_type = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime_type.as_ref())
                .body(Body::from(file.contents().to_vec()))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap(),
    }
}

async fn incoming_stream_list_handler(
    ws: WebSocketUpgrade,
    AxumState(state): AxumState<Arc<Mutex<State>>>,
) -> impl axum::response::IntoResponse {
    let peer_id = Uuid::new_v4();
    info!("[{}] New WebSocket connection attempt", peer_id);
    ws.on_upgrade(move |socket| handle_stream_list_socket(state, socket, peer_id))
}

async fn handle_stream_list_socket(state: Arc<Mutex<State>>, ws: WebSocket, peer_id: Uuid) {
    info!(
        "WebSocket connection established with ID {} for stream list.",
        peer_id
    );
    let (mut sink, mut stream) = ws.split();
    let mut ping_interval = interval(Duration::from_secs(30));
    let mut receiver = {
        let mut state = state.lock().await;
        state.get_streams_changed_receiver()
    };

    {
        let state = state.lock().await;
        let stream_ids = state.get_stream_ids();
        if let Err(e) = sink
            .send(Message::Text(
                serde_json::to_string(&stream_ids).unwrap().into(),
            ))
            .await
        {
            error!("[{}] Failed to send initial stream list: {}", peer_id, e);
            return;
        }
    }

    loop {
        select! {
            ws_msg_option = stream.next() => {
                match ws_msg_option {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Text(t) => warn!("[{}] Received unexpected Text: {}", peer_id, t),
                            Message::Binary(_) => warn!("[{}] Received unexpected Binary", peer_id),
                            Message::Ping(p) => {
                                info!("[{}] Received Ping from client", peer_id);
                                if let Err(e) = sink.send(Message::Pong(p)).await {
                                    error!("[{}] Failed to send Pong: {}", peer_id, e);
                                    break;
                                }
                            },
                            Message::Pong(_) => {},
                            Message::Close(c) => {
                                if let Some(cf) = c {
                                    info!("[{}] Received Close from client: Code={}, Reason='{}'", peer_id, cf.code, cf.reason);
                                } else {
                                    info!("[{}] Received Close from client (no code/reason)", peer_id);
                                }
                                break;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        error!("[{}] WebSocket receive error: {}", peer_id, e);
                        break;
                    }
                    None => {
                        info!("[{}] WebSocket connection closed by client", peer_id);
                        break;
                    }
                }
            }

            recv = receiver.recv() => {
                match recv {
                    Ok(stream_ids) => {
                        if let Err(e) = sink.send(Message::Text(serde_json::to_string(&stream_ids).unwrap().into())).await {
                             error!("[{}] Failed to send new stream list to WebSocket: {}", peer_id, e);
                             break;
                        }
                    }
                    Err(RecvError::Lagged(count)) => {
                        warn!("[{}] Lagged for stream list! Dropping {} messages and resending list.", peer_id, count);
                        let state = state.lock().await;
                        let stream_ids = state.get_stream_ids();
                        if let Err(e) = sink.send(Message::Text(serde_json::to_string(&stream_ids).unwrap().into())).await {
                             error!("[{}] Failed to resend stream list: {}", peer_id, e);
                             break;
                        }
                    }
                    Err(RecvError::Closed) => {
                        error!("[{}] Broadcast sender for stream list closed.", peer_id);
                        break;
                    }
                }
            }

             _ = ping_interval.tick() => {
                 if sink.send(Message::Ping(vec![].into())).await.is_err() {
                     error!("[{}] Failed to send Ping. Closing.", peer_id);
                     break;
                 }
            }
        }
    }
}

async fn incoming_stream_handler(
    ws: WebSocketUpgrade,
    AxumState(state): AxumState<Arc<Mutex<State>>>,
    Path(stream_id): Path<String>,
) -> impl axum::response::IntoResponse {
    let peer_id = Uuid::new_v4();
    info!("[{}] New WebSocket connection attempt", peer_id);
    ws.on_upgrade(move |socket| handle_stream_socket(state, socket, peer_id, stream_id))
}

async fn handle_stream_socket(
    state: Arc<Mutex<State>>,
    ws: WebSocket,
    peer_id: Uuid,
    stream_id: String,
) {
    info!(
        "WebSocket connection established with ID {} for stream ID {}",
        peer_id, stream_id
    );
    let (mut sink, mut stream) = ws.split();
    let mut ping_interval = interval(Duration::from_secs(30));
    let mut receiver = {
        let mut state = state.lock().await;
        if let Some(receiver) = state.get_stream_receiver(&stream_id) {
            receiver
        } else {
            error!(
                "[{}] Stream ID '{}' not found, closing connection.",
                peer_id, stream_id
            );
            return;
        }
    };

    loop {
        select! {
            ws_msg_option = stream.next() => {
                match ws_msg_option {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Text(t) => warn!("[{}] Received unexpected Text: {}", peer_id, t),
                            Message::Binary(_) => warn!("[{}] Received unexpected Binary", peer_id),
                            Message::Ping(p) => {
                                info!("[{}] Received Ping from client", peer_id);
                                if let Err(e) = sink.send(Message::Pong(p)).await {
                                    error!("[{}] Failed to send Pong: {}", peer_id, e);
                                    break;
                                }
                            },
                            Message::Pong(_) => {},
                            Message::Close(c) => {
                                if let Some(cf) = c {
                                    info!("[{}] Received Close from client: Code={}, Reason='{}'", peer_id, cf.code, cf.reason);
                                } else {
                                    info!("[{}] Received Close from client (no code/reason)", peer_id);
                                }
                                break;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        error!("[{}] WebSocket receive error: {}", peer_id, e);
                        break;
                    }
                    None => {
                        info!("[{}] WebSocket connection closed by client", peer_id);
                        break;
                    }
                }
            }

            recv = receiver.recv() => {
                match recv {
                    Ok(packet) => {
                        if let Err(e) = sink.send(Message::Binary(packet)).await {
                             error!("[{}] Failed to send frame for stream '{}' to WebSocket: {}", peer_id, stream_id, e);
                             break;
                        }
                    }
                    Err(RecvError::Lagged(count)) => {
                        warn!("[{}] Lagged for stream '{}'! Skipped {} messages.", peer_id, stream_id, count);
                    }
                    Err(RecvError::Closed) => {
                        info!("[{}] Broadcast sender for stream '{}' closed.", peer_id, stream_id);
                        break;
                    }
                }
            }

             _ = ping_interval.tick() => {
                 if sink.send(Message::Ping(vec![].into())).await.is_err() {
                     error!("[{}] Failed to send Ping. Closing.", peer_id);
                     break;
                 }
            }
        }
    }
}
