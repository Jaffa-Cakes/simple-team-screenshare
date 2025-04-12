use std::{io, net::SocketAddr};

use futures::join;
use state::State;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod srt;
mod state;
mod websocket;

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let state = State::new();

    join!(
        srt::srt_listen("0.0.0.0:7092", state.clone()),
        websocket::websocket_listen(SocketAddr::from(([0, 0, 0, 0], 7091)), state),
    );

    Ok(())
}
