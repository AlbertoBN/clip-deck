//! The daemon IPC client, behind a trait so Tauri commands are testable
//! against a fake instead of a real Unix socket connection to `clipd`.

use clip_ipc::protocol::{Command, Event, Response};
use tokio::sync::broadcast;

#[async_trait::async_trait]
pub trait Client: Send + Sync {
    async fn call(&self, command: Command) -> Response;
    fn subscribe(&self) -> broadcast::Receiver<Event>;
}

#[async_trait::async_trait]
impl Client for clip_ipc::client::IpcClient {
    async fn call(&self, command: Command) -> Response {
        clip_ipc::client::IpcClient::call(self, command).await
    }

    fn subscribe(&self) -> broadcast::Receiver<Event> {
        clip_ipc::client::IpcClient::subscribe(self)
    }
}

/// A cloneable handle to the managed client, held in Tauri's state.
pub struct ClientHandle(pub std::sync::Arc<dyn Client>);

/// Subscribes to the client's daemon event stream and forwards each event to
/// `emit`, until the client's broadcast channel closes.
pub fn spawn_event_forwarder(client: std::sync::Arc<dyn Client>, emit: impl Fn(Event) + Send + Sync + 'static) {
    tokio::spawn(async move {
        let mut events = client.subscribe();
        while let Ok(event) = events.recv().await {
            emit(event);
        }
    });
}

#[cfg(test)]
pub(crate) mod fakes {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    pub(crate) struct FakeClient {
        responses: Mutex<VecDeque<Response>>,
        calls: Mutex<Vec<Command>>,
        events_tx: broadcast::Sender<Event>,
    }

    impl FakeClient {
        pub(crate) fn new() -> Self {
            let (events_tx, _rx) = broadcast::channel(16);
            Self { responses: Mutex::new(VecDeque::new()), calls: Mutex::new(Vec::new()), events_tx }
        }

        pub(crate) fn push_response(&self, response: Response) {
            self.responses.lock().unwrap().push_back(response);
        }

        pub(crate) fn calls(&self) -> Vec<Command> {
            self.calls.lock().unwrap().clone()
        }

        pub(crate) fn publish_event(&self, event: Event) {
            let _ = self.events_tx.send(event);
        }
    }

    #[async_trait::async_trait]
    impl Client for FakeClient {
        async fn call(&self, command: Command) -> Response {
            self.calls.lock().unwrap().push(command);
            self.responses.lock().unwrap().pop_front().unwrap_or_else(|| Response::err("test", "no canned response queued"))
        }

        fn subscribe(&self) -> broadcast::Receiver<Event> {
            self.events_tx.subscribe()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fakes::FakeClient;
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn daemon_events_are_forwarded_to_the_emitter() {
        let client = Arc::new(FakeClient::new());
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let emitted_clone = emitted.clone();

        spawn_event_forwarder(client.clone(), move |event| emitted_clone.lock().unwrap().push(event));
        tokio::task::yield_now().await;

        client.publish_event(Event::HotkeyPressed);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        assert_eq!(emitted.lock().unwrap().clone(), vec![Event::HotkeyPressed]);
    }
}
