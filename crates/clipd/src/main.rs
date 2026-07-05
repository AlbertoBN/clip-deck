//! Startup and lifecycle.

mod app;
mod commands;
mod ingest;
mod jobs;
mod telemetry;
mod watch_loop;

fn main() {
    todo!("wire up daemon startup: config load, storage init, IPC server, watch loop")
}
