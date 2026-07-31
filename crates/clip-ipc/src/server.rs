//! Daemon-side Unix socket server.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, Mutex};

use crate::auth::{CurrentUidCheck, PeerCredentialCheck};
use crate::protocol::{Command, Event, Request, Response, ServerMessage};

/// Lines longer than this are rejected and the connection is closed, to
/// bound per-connection memory use.
const MAX_LINE_LEN: usize = 1024 * 1024;

/// A registered command handler: takes a decoded `Command`, returns the
/// resulting JSON payload or an error message.
pub type HandlerFn = Arc<
    dyn Fn(Command) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>> + Send + Sync,
>;

/// A cloneable handle for publishing events to every connected client.
#[derive(Clone)]
pub struct EventPublisher {
    tx: broadcast::Sender<Event>,
}

impl EventPublisher {
    pub fn publish(&self, event: Event) {
        // No receivers connected yet is not an error - events are best-effort broadcast.
        let _ = self.tx.send(event);
    }
}

pub struct Server {
    listener: UnixListener,
    handler: HandlerFn,
    events_tx: broadcast::Sender<Event>,
    peer_check: Arc<dyn PeerCredentialCheck>,
}

impl Server {
    /// Binds a Unix socket at `socket_path`, removing any stale leftover
    /// socket file first, restricting its permissions, and enforcing the
    /// default (current-user-only) peer credential check.
    pub fn bind(socket_path: &Path, handler: HandlerFn) -> std::io::Result<Self> {
        Self::bind_with_check(socket_path, handler, Arc::new(CurrentUidCheck))
    }

    /// Like [`Server::bind`], but with an injectable peer credential check
    /// (used by tests to simulate a rejected connection without actually
    /// running as two different users).
    pub fn bind_with_check(
        socket_path: &Path,
        handler: HandlerFn,
        peer_check: Arc<dyn PeerCredentialCheck>,
    ) -> std::io::Result<Self> {
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }
        let listener = UnixListener::bind(socket_path)?;
        crate::auth::secure_permissions(socket_path)?;
        let (events_tx, _rx) = broadcast::channel(256);
        Ok(Self { listener, handler, events_tx, peer_check })
    }

    /// A cloneable handle other components can use to publish events.
    pub fn event_publisher(&self) -> EventPublisher {
        EventPublisher { tx: self.events_tx.clone() }
    }

    /// Accepts connections in a loop, spawning a task per connection, until
    /// the listener errors.
    pub async fn run(self) -> std::io::Result<()> {
        loop {
            let (stream, _addr) = self.listener.accept().await?;
            let handler = self.handler.clone();
            let events_rx = self.events_tx.subscribe();
            let peer_check = self.peer_check.clone();
            tokio::spawn(async move {
                if let Ok(cred) = stream.peer_cred() {
                    if !peer_check.is_allowed(cred.uid()) {
                        return; // Reject silently: close without dispatching any command.
                    }
                }
                handle_connection(stream, handler, events_rx, None).await;
            });
        }
    }

    /// Like [`Server::run`], but stops accepting new connections once
    /// `shutdown` resolves, then waits for every currently-executing command
    /// handler to finish (and its response to be written) before returning -
    /// so an in-flight command completes rather than being cut off
    /// mid-response. Connections that are merely open and idle (waiting for
    /// their next request) are dropped once the drain completes; they are
    /// not "in-flight work".
    pub async fn run_with_shutdown(self, shutdown: impl std::future::Future<Output = ()>) -> std::io::Result<()> {
        tokio::pin!(shutdown);
        let mut tasks = tokio::task::JoinSet::new();
        let in_flight = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        loop {
            tokio::select! {
                accept_result = self.listener.accept() => {
                    let (stream, _addr) = accept_result?;
                    let handler = self.handler.clone();
                    let events_rx = self.events_tx.subscribe();
                    let peer_check = self.peer_check.clone();
                    let in_flight = in_flight.clone();
                    tasks.spawn(async move {
                        if let Ok(cred) = stream.peer_cred() {
                            if !peer_check.is_allowed(cred.uid()) {
                                return;
                            }
                        }
                        handle_connection(stream, handler, events_rx, Some(in_flight)).await;
                    });
                }
                _ = &mut shutdown => break,
            }
        }
        while in_flight.load(std::sync::atomic::Ordering::SeqCst) > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        Ok(())
    }
}

async fn write_message(writer: &Mutex<tokio::net::unix::OwnedWriteHalf>, message: &ServerMessage) -> std::io::Result<()> {
    let line = serde_json::to_string(message).expect("ServerMessage always serializes") + "\n";
    let mut w = writer.lock().await;
    w.write_all(line.as_bytes()).await
}

async fn handle_connection(
    stream: UnixStream,
    handler: HandlerFn,
    mut events_rx: broadcast::Receiver<Event>,
    in_flight: Option<Arc<std::sync::atomic::AtomicUsize>>,
) {
    let (read_half, write_half) = stream.into_split();
    let writer = Arc::new(Mutex::new(write_half));

    let event_writer = writer.clone();
    let event_task = tokio::spawn(async move {
        while let Ok(event) = events_rx.recv().await {
            if write_message(&event_writer, &ServerMessage::Event(event)).await.is_err() {
                break;
            }
        }
    });

    let mut reader = BufReader::new(read_half);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF: client disconnected
            Ok(_) => {
                if line.len() > MAX_LINE_LEN {
                    let _ = write_message(
                        &writer,
                        &ServerMessage::Response(Response::err("", "line too long")),
                    )
                    .await;
                    break;
                }
                let trimmed = line.trim_end();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<Request>(trimmed) {
                    Ok(request) => {
                        if let Some(counter) = &in_flight {
                            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        }
                        let result = (handler)(request.command).await;
                        let response = match result {
                            Ok(payload) => Response::ok(request.request_id, payload),
                            Err(error) => Response::err(request.request_id, error),
                        };
                        let write_result = write_message(&writer, &ServerMessage::Response(response)).await;
                        if let Some(counter) = &in_flight {
                            counter.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                        }
                        if write_result.is_err() {
                            break;
                        }
                    }
                    Err(_) => continue,
                }
            }
            Err(_) => break,
        }
    }
    event_task.abort();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Command, Response};
    use std::sync::Arc;

    fn echo_handler() -> HandlerFn {
        Arc::new(|command: Command| {
            Box::pin(async move {
                match command {
                    Command::GetSettings => Ok(serde_json::json!({"ok": true})),
                    _ => Err("unsupported in test".to_string()),
                }
            })
        })
    }

    #[tokio::test]
    async fn server_binds_successfully_on_a_fresh_runtime_directory() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        let _server = Server::bind(&socket_path, echo_handler()).unwrap();
        assert!(socket_path.exists());
    }

    #[tokio::test]
    async fn server_binds_successfully_over_a_stale_socket_file() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        std::fs::write(&socket_path, b"stale").unwrap();
        let result = Server::bind(&socket_path, echo_handler());
        assert!(result.is_ok());
    }

    async fn send_request(stream: &mut tokio::net::UnixStream, request: &crate::protocol::Request) {
        use tokio::io::AsyncWriteExt;
        let line = serde_json::to_string(request).unwrap() + "\n";
        stream.write_all(line.as_bytes()).await.unwrap();
    }

    async fn recv_response(reader: &mut tokio::io::BufReader<tokio::net::unix::OwnedReadHalf>) -> Response {
        use tokio::io::AsyncBufReadExt;
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        match serde_json::from_str::<ServerMessage>(&line).unwrap() {
            ServerMessage::Response(r) => r,
            ServerMessage::Event(e) => panic!("expected a Response, got event {e:?}"),
        }
    }

    #[tokio::test]
    async fn two_clients_connect_at_the_same_time() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        let server = Server::bind(&socket_path, echo_handler()).unwrap();
        tokio::spawn(server.run());

        let mut client_a = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
        let mut client_b = tokio::net::UnixStream::connect(&socket_path).await.unwrap();

        send_request(&mut client_a, &crate::protocol::Request::new("a1", Command::GetSettings)).await;
        send_request(&mut client_b, &crate::protocol::Request::new("b1", Command::GetSettings)).await;

        let (read_a, _write_a) = client_a.into_split();
        let (read_b, _write_b) = client_b.into_split();
        let mut reader_a = tokio::io::BufReader::new(read_a);
        let mut reader_b = tokio::io::BufReader::new(read_b);

        let resp_a = recv_response(&mut reader_a).await;
        let resp_b = recv_response(&mut reader_b).await;
        assert_eq!(resp_a.request_id(), "a1");
        assert_eq!(resp_b.request_id(), "b1");
    }

    #[tokio::test]
    async fn response_envelope_carries_the_original_request_id() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        let server = Server::bind(&socket_path, echo_handler()).unwrap();
        tokio::spawn(server.run());

        let mut client = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
        send_request(&mut client, &crate::protocol::Request::new("r1", Command::GetSettings)).await;
        let (read, _write) = client.into_split();
        let mut reader = tokio::io::BufReader::new(read);
        let response = recv_response(&mut reader).await;
        assert_eq!(response.request_id(), "r1");
    }

    #[tokio::test]
    async fn handler_error_yields_err_response_and_keeps_connection_open() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        let server = Server::bind(&socket_path, echo_handler()).unwrap();
        tokio::spawn(server.run());

        let mut client = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
        send_request(&mut client, &crate::protocol::Request::new("r1", Command::ListRules)).await;
        send_request(&mut client, &crate::protocol::Request::new("r2", Command::GetSettings)).await;

        let (read, _write) = client.into_split();
        let mut reader = tokio::io::BufReader::new(read);
        let first = recv_response(&mut reader).await;
        assert!(matches!(first, Response::Err { .. }));
        let second = recv_response(&mut reader).await;
        assert_eq!(second.request_id(), "r2");
        assert!(matches!(second, Response::Ok { .. }));
    }

    #[tokio::test]
    async fn event_triggered_by_one_client_is_seen_by_another_client() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        let server = Server::bind(&socket_path, echo_handler()).unwrap();
        let publisher = server.event_publisher();
        tokio::spawn(server.run());

        let client_b = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
        // Give client B's connection task a moment to register its broadcast subscription.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        publisher.publish(crate::protocol::Event::ClipUpdated { clip_id: "c1".to_string() });

        let (read_b, _write_b) = client_b.into_split();
        let mut reader_b = tokio::io::BufReader::new(read_b);
        use tokio::io::AsyncBufReadExt;
        let mut line = String::new();
        reader_b.read_line(&mut line).await.unwrap();
        let message: ServerMessage = serde_json::from_str(&line).unwrap();
        match message {
            ServerMessage::Event(crate::protocol::Event::ClipUpdated { clip_id }) => {
                assert_eq!(clip_id, "c1");
            }
            other => panic!("expected ClipUpdated event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn an_over_long_line_is_rejected_and_the_connection_is_closed() {
        use tokio::io::AsyncWriteExt;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        let server = Server::bind(&socket_path, echo_handler()).unwrap();
        tokio::spawn(server.run());

        let mut client = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
        let huge_line = "x".repeat(MAX_LINE_LEN + 1);
        client.write_all(huge_line.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();

        let (read, mut write) = client.into_split();
        let mut reader = tokio::io::BufReader::new(read);
        let response = recv_response(&mut reader).await;
        assert!(matches!(response, Response::Err { .. }));

        // Connection should now be closed by the server: further writes eventually fail,
        // or reading returns EOF (0 bytes) rather than another message.
        use tokio::io::AsyncBufReadExt;
        let _ = write.write_all(b"\n").await;
        let mut trailing = String::new();
        let n = reader.read_line(&mut trailing).await.unwrap_or(0);
        assert_eq!(n, 0, "expected EOF after the server closed the connection");
    }

    #[tokio::test]
    async fn run_with_shutdown_drains_an_in_flight_handler_before_returning() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        let slow_handler: HandlerFn = Arc::new(|_command: Command| {
            Box::pin(async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                Ok(serde_json::json!({"ok": true}))
            })
        });
        let server = Server::bind(&socket_path, slow_handler).unwrap();

        let mut client = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
        send_request(&mut client, &crate::protocol::Request::new("r1", Command::GetSettings)).await;

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let run_handle = tokio::spawn(server.run_with_shutdown(async {
            let _ = shutdown_rx.await;
        }));

        // Give the accept loop time to accept and dispatch the request, then
        // signal shutdown while the handler is still sleeping.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let _ = shutdown_tx.send(());

        run_handle.await.unwrap().unwrap();

        let (read, _write) = client.into_split();
        let mut reader = tokio::io::BufReader::new(read);
        let response = recv_response(&mut reader).await;
        assert_eq!(response.request_id(), "r1");
    }
}
