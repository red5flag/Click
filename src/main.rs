use anyhow::Result;
use std::future::IntoFuture;
use tracing::{error, info};

use person_detect::app::Application;
use person_detect::config::Config;
use person_detect::web::{create_router, WebState};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("person_detect=info".parse().unwrap())
                .add_directive("tract_onnx=warn".parse().unwrap()),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("Person Detection System Starting");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::load().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    info!("Configuration loaded successfully");
    info!("Camera index: {}", config.camera_index);
    info!("Confidence threshold: {}", config.confidence_threshold);
    info!("IoU threshold: {}", config.iou_threshold);
    info!("Grace period: {} seconds", config.grace_period_seconds);
    info!("Recordings directory: {}", config.recordings_directory);

    // Shared web state
    let web_state = WebState::new();

    // Create and run application
    let app = Application::new(config, web_state.clone()).await?;
    let shutdown_handle = app.get_shutdown_handle();

    // Start web server
    let server_handle = tokio::spawn(start_web_server(web_state, shutdown_handle.clone()));

    // Setup shutdown handlers
    let shutdown_signal = shutdown_handle.clone();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigint = signal(SignalKind::interrupt()).expect("Failed to create SIGINT handler");
            let mut sigterm = signal(SignalKind::terminate()).expect("Failed to create SIGTERM handler");

            tokio::select! {
                _ = sigint.recv() => info!("Received SIGINT, initiating shutdown"),
                _ = sigterm.recv() => info!("Received SIGTERM, initiating shutdown"),
            }
        }

        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c().await.expect("Failed to create Ctrl+C handler");
            info!("Received Ctrl+C, initiating shutdown");
        }

        shutdown_signal.shutdown().await;
    });

    // Run the detection application
    let app_result = app.run().await;

    // Ensure server shuts down when detection app stops
    shutdown_handle.shutdown().await;
    let _ = server_handle.await;

    if let Err(e) = app_result {
        error!("Application error: {}", e);
        return Err(e);
    }

    info!("Application shutdown complete");
    Ok(())
}

async fn start_web_server(state: WebState, shutdown: person_detect::app::ShutdownHandle) -> Result<()> {
    let app = create_router(state);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Web server listening on http://{}", addr);

    tokio::select! {
        result = axum::serve(listener, app).into_future() => {
            result?;
        }
        _ = shutdown.wait_for_shutdown() => {
            info!("Web server received shutdown signal");
        }
    }

    Ok(())
}

