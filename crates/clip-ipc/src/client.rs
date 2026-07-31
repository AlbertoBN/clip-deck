//! UI-side client.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio::task::JoinHandle;

use crate::protocol::{Command, Event, Request, Response, ServerMessage};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("daemon not running")]
    DaemonNotRunning,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<Response>>>>;

/// UI-side IPC client: sends commands and awaits their correlated response,
/// and exposes a subscription stream of broadcast events.
pub struct IpcClient {
    write_half: Mutex<tokio::net::unix::OwnedWriteHalf>,
    pending: PendingMap,
    events_tx: broadcast::Sender<Event>,
    _reader_task: JoinHandle<()>,
}

impl IpcClient {
    /// Connects to the daemon's Unix socket at `socket_path`.
    pub async fn connect(socket_path: &Path) -> Result<Self, ClientError> {
        let stream = UnixStream::connect(socket_path).await.map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused => ClientError::DaemonNotRunning,
            _ => ClientError::Io(e),
        })?;
        let (read_half, write_half) = stream.into_split();
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (events_tx, _rx) = broadcast::channel(256);

        let pending_for_reader = pending.clone();
        let events_tx_for_reader = events_tx.clone();
        let reader_task = tokio::spawn(async move {
            let mut reader = BufReader::new(read_half);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        if let Ok(message) = serde_json::from_str::<ServerMessage>(line.trim_end()) {
                            match message {
                                ServerMessage::Response(response) => {
                                    let id = response.request_id().to_string();
                                    let mut map = pending_for_reader.lock().await;
                                    if let Some(sender) = map.remove(&id) {
                                        let _ = sender.send(response);
                                    }
                                }
                                ServerMessage::Event(event) => {
                                    let _ = events_tx_for_reader.send(event);
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self { write_half: Mutex::new(write_half), pending, events_tx, _reader_task: reader_task })
    }

    /// Sends a command and awaits its correlated response.
    pub async fn call(&self, command: Command) -> Response {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        {
            let mut map = self.pending.lock().await;
            map.insert(request_id.clone(), tx);
        }
        let request = Request::new(request_id, command);
        let line = serde_json::to_string(&request).expect("Request always serializes") + "\n";
        {
            let mut w = self.write_half.lock().await;
            let _ = w.write_all(line.as_bytes()).await;
        }
        rx.await.expect("connection closed before a response arrived")
    }

    /// Subscribes to the broadcast event stream.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Command, Event, Response};
    use crate::server::Server;
    use std::sync::Arc;

    fn dispatch_handler() -> crate::server::HandlerFn {
        Arc::new(|command: Command| {
            Box::pin(async move {
                match command {
                    Command::GetSettings => Ok(serde_json::json!({"which": "settings"})),
                    Command::ListRules => Ok(serde_json::json!({"which": "rules"})),
                    _ => Err("unsupported in test".to_string()),
                }
            })
        })
    }

    async fn start_test_server() -> (std::path::PathBuf, tempfile::TempDir, crate::server::EventPublisher) {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        let server = Server::bind(&socket_path, dispatch_handler()).unwrap();
        let publisher = server.event_publisher();
        tokio::spawn(server.run());
        (socket_path, dir, publisher)
    }

    #[tokio::test]
    async fn client_receives_the_response_matching_its_request() {
        let (socket_path, _dir, _publisher) = start_test_server().await;
        let client = IpcClient::connect(&socket_path).await.unwrap();

        let (settings_resp, rules_resp) =
            tokio::join!(client.call(Command::GetSettings), client.call(Command::ListRules));

        match settings_resp {
            Response::Ok { payload, .. } => assert_eq!(payload, serde_json::json!({"which": "settings"})),
            Response::Err { error, .. } => panic!("unexpected error: {error}"),
        }
        match rules_resp {
            Response::Ok { payload, .. } => assert_eq!(payload, serde_json::json!({"which": "rules"})),
            Response::Err { error, .. } => panic!("unexpected error: {error}"),
        }
    }

    #[tokio::test]
    async fn client_observes_a_hotkey_pressed_event() {
        let (socket_path, _dir, publisher) = start_test_server().await;
        let client = IpcClient::connect(&socket_path).await.unwrap();
        let mut events = client.subscribe();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        publisher.publish(Event::HotkeyPressed);

        let event = events.recv().await.unwrap();
        assert_eq!(event, Event::HotkeyPressed);
    }

    #[tokio::test]
    async fn connecting_to_a_nonexistent_socket_returns_a_distinguishable_error() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("no-such-daemon.sock");
        let result = IpcClient::connect(&socket_path).await;
        assert!(matches!(result, Err(ClientError::DaemonNotRunning)));
    }
}
