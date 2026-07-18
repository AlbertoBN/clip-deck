//! Local-socket scope and single-user protections.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Restricts the socket's containing directory to `0700` and the socket file
/// itself to `0600`, so no other local account can open it at the
/// filesystem level.
pub fn secure_permissions(socket_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    }
    std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// Defense-in-depth peer-UID check, evaluated per accepted connection.
pub trait PeerCredentialCheck: Send + Sync {
    fn is_allowed(&self, peer_uid: u32) -> bool;
}

/// The production check: only the current process's own UID may connect.
pub struct CurrentUidCheck;

impl PeerCredentialCheck for CurrentUidCheck {
    fn is_allowed(&self, peer_uid: u32) -> bool {
        peer_uid == unsafe { libc::getuid() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Command, Response};
    use crate::server::Server;
    use std::sync::Arc;

    fn echo_handler() -> crate::server::HandlerFn {
        Arc::new(|_command: Command| Box::pin(async move { Ok(serde_json::json!({"ok": true})) }))
    }

    #[test]
    fn socket_file_is_not_group_or_world_accessible() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        std::fs::write(&socket_path, b"placeholder").unwrap();
        secure_permissions(&socket_path).unwrap();
        let mode = std::fs::metadata(&socket_path).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "group/other bits should be clear, got mode {mode:o}");
    }

    struct AlwaysAllow;
    impl PeerCredentialCheck for AlwaysAllow {
        fn is_allowed(&self, _peer_uid: u32) -> bool {
            true
        }
    }

    struct AlwaysDeny;
    impl PeerCredentialCheck for AlwaysDeny {
        fn is_allowed(&self, _peer_uid: u32) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn connection_from_the_same_uid_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        let server = Server::bind_with_check(&socket_path, echo_handler(), Arc::new(AlwaysAllow)).unwrap();
        tokio::spawn(server.run());

        let mut client = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
        use tokio::io::AsyncWriteExt;
        let request = crate::protocol::Request::new("r1", Command::GetSettings);
        let line = serde_json::to_string(&request).unwrap() + "\n";
        client.write_all(line.as_bytes()).await.unwrap();

        let (read, _write) = client.into_split();
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(read);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.unwrap();
        let message: crate::protocol::ServerMessage = serde_json::from_str(&response_line).unwrap();
        match message {
            crate::protocol::ServerMessage::Response(Response::Ok { request_id, .. }) => {
                assert_eq!(request_id, "r1");
            }
            other => panic!("expected an Ok response, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn connection_from_a_different_uid_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("clipd.sock");
        let server = Server::bind_with_check(&socket_path, echo_handler(), Arc::new(AlwaysDeny)).unwrap();
        tokio::spawn(server.run());

        let mut client = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
        use tokio::io::AsyncWriteExt;
        let request = crate::protocol::Request::new("r1", Command::GetSettings);
        let line = serde_json::to_string(&request).unwrap() + "\n";
        // The server may reject the connection immediately (before or while this write
        // happens), so a write error here is itself evidence of rejection, not a test bug.
        let _ = client.write_all(line.as_bytes()).await;

        let (read, _write) = client.into_split();
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(read);
        let mut response_line = String::new();
        match reader.read_line(&mut response_line).await {
            Ok(0) => {} // EOF: connection closed without a response - expected.
            Ok(_) => panic!("expected no response, but got: {response_line}"),
            Err(_) => {} // Connection reset before any data - also expected.
        }
    }
}
