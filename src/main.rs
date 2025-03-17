mod utils;

use axum::{routing::post, Router};
use clap::Parser;
use rodio::OutputStreamHandle;
use std::{
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use utils::args::Args;
use utils::playback::{get_output_stream, list_host_devices};

use utils::handler::{update,shutdown_signal};

struct AppState {
    ply_name: String,
    ply_kills: u16,
    stream_handle: Arc<OutputStreamHandle>,
    args: Arc<Args>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,tower_http=debug,axum=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let args = Args::parse();

    if args.list_devices {
        list_host_devices();
        return;
    }

    // initialize the specified audio device
    let output_stream = get_output_stream(&args.device);

    let app_state = Arc::new(Mutex::new(AppState {
        ply_name: "".to_string(),
        ply_kills: 0,
        stream_handle: Arc::new(output_stream.1),
        args: Arc::new(args),
    }));

    let app = Router::new()
        .route("/", post(update))
        .with_state(app_state)
        .layer((
            TraceLayer::new_for_http(),
            // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
            // requests don't hang forever.
            TimeoutLayer::new(Duration::from_secs(10)),
        ));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
