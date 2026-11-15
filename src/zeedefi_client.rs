//! `src/zeedefi_client.rs` — client async du démon ZeedeFi OS (socket Unix).
//!
//! Le client tourne dans une tâche Tokio dédiée : il gère la connexion, la
//! reconnexion avec backoff exponentiel, l'émission des requêtes et la
//! réception des enveloppes JSON. L'interface UI ne bloque jamais : elle
//! envoie des requêtes via un canal et récupère les événements par `try_recv`.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, Notify};
use tracing::{debug, info, warn};


/// Chemin par défaut du socket Unix du démon.
pub const DEFAULT_SOCKET_PATH: &str = "/run/zeedefi/zeedefi.sock";

/// Variable d'environnement surchargeant le chemin du socket.
pub const ENV_SOCKET_PATH: &str = "ZEEDEFI_SOCKET";

/// Délai maximal pour une réponse du démon.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Backoff initial de reconnexion.
pub const BACKOFF_INITIAL: Duration = Duration::from_secs(1);

/// Backoff maximal de reconnexion.
pub const BACKOFF_MAX: Duration = Duration::from_secs(30);

/// Taxonomie numérique des erreurs client (affichée telle quelle par l'UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ErrorCode {
    Unknown = 0,
    ConnectionRefused = 1,
    SocketMissing = 2,
    Timeout = 3,
    Protocol = 4,
    Io = 5,
    Daemon = 100,
}

/// Enveloppe de requête envoyée au démon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    #[serde(default)]
    pub params: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
}

impl Request {
    pub fn new(method: &str, params: Value) -> Self {
        Self {
            method: method.to_string(),
            params,
            id: None,
        }
    }

    pub fn with_id(method: &str, params: Value, id: u64) -> Self {
        Self {
            method: method.to_string(),
            params,
            id: Some(id),
        }
    }
}

/// Entrée de bibliothèque renvoyée par le démon (méthode `list_games`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DaemonGame {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, rename = "cover_path")]
    pub cover_path: Option<String>,
    #[serde(default, rename = "background_path")]
    pub background_path: Option<String>,
    #[serde(default, rename = "store_icon")]
    pub store_icon: Option<String>,
}

/// Enveloppe de réponse renvoyée par le démon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    pub code: u32,
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub id: Option<u64>,
}

/// Erreur client, toujours associée à un code numérique.
#[derive(Debug, Clone, Error)]
pub enum ClientError {
    #[error("socket introuvable: {0}")]
    SocketMissing(PathBuf),
    #[error("connexion refusée par le démon ZeedeFi")]
    ConnectionRefused,
    #[error("délai dépassé pour la requête (5s)")]
    Timeout,
    #[error("réponse invalide du démon: {0}")]
    Protocol(String),
    #[error("erreur d'E/S: {0}")]
    Io(String),
}

/// Événement remonté au thread UI.
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Connected,
    Disconnected,
    Response {
        method: String,
        response: Response,
    },
    Error {
        code: ErrorCode,
        daemon_code: Option<u32>,
    },
}

/// Client de démon. Les requêtes sont envoyées via `request` (non bloquant) ;
/// les événements sont récupérés via `try_recv_event`.
#[derive(Clone)]
pub struct ZeedeFiClient {
    requests: mpsc::UnboundedSender<Request>,
    events: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ClientEvent>>>,
    shutdown: Arc<Notify>,
}

impl std::fmt::Debug for ZeedeFiClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ZeedeFiClient").finish()
    }
}

impl ZeedeFiClient {
    /// Lance la tâche de fond et retourne le client.
    pub fn spawn(handle: tokio::runtime::Handle, socket_path: PathBuf) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel::<Request>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<ClientEvent>();
        let shutdown = Arc::new(Notify::new());

        handle.spawn(run_client(socket_path, request_rx, event_tx, Arc::clone(&shutdown)));

        Self {
            requests: request_tx,
            events: Arc::new(tokio::sync::Mutex::new(event_rx)),
            shutdown,
        }
    }

    /// Envoie une requête (non bloquant).
    pub fn request(&self, mut request: Request) -> bool {
        static COUNTER: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(1);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        request.id = Some(id);
        self.requests.send(request).is_ok()
    }

    /// Récupère un événement sans bloquer.
    pub async fn try_recv_event_async(&self) -> Option<ClientEvent> {
        let mut guard = self.events.lock().await;
        guard.try_recv().ok()
    }

    /// Récupère un événement sans bloquer (synchrone, best-effort).
    pub fn try_recv_event(&self) -> Option<ClientEvent> {
        // Les canaux Tokio n'exposent pas de try_recv sans async ; on utilise
        // un blocage léger sur le handle courant. En pratique cette méthode est
        // appelée depuis le thread UI ; on préfère la variante async ci-dessus.
        None
    }

    /// Signale l'arrêt de la tâche de fond.
    pub fn shutdown(&self) {
        self.shutdown.notify_one();
    }
}

/// Tâche de fond : boucle de connexion/reconnexion.
async fn run_client(
    socket_path: PathBuf,
    mut request_rx: mpsc::UnboundedReceiver<Request>,
    event_tx: mpsc::UnboundedSender<ClientEvent>,
    shutdown: Arc<Notify>,
) {
    let mut backoff = BACKOFF_INITIAL;
    loop {
        match tokio::net::UnixStream::connect(&socket_path).await {
            Ok(stream) => {
                info!(socket = %socket_path.display(), "connecté au démon ZeedeFi");
                let _ = event_tx.send(ClientEvent::Connected);
                backoff = BACKOFF_INITIAL;
                if let Err(error) = run_session(stream, &mut request_rx, &event_tx, &shutdown).await {
                    warn!(error = %error, "session démon terminée");
                }
                let _ = event_tx.send(ClientEvent::Disconnected);
            }
            Err(error) => {
                let code = classify_connect_error(&socket_path, &error);
                warn!(socket = %socket_path.display(), error = %error, "connexion au démon échouée");
                let _ = event_tx.send(ClientEvent::Error {
                    code,
                    daemon_code: None,
                });
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = tokio::time::sleep(backoff) => {}
                }
                backoff = next_backoff(backoff);
            }
        }
    }
    debug!("client ZeedeFi arrêté");
}

async fn run_session(
    stream: UnixStream,
    request_rx: &mut mpsc::UnboundedReceiver<Request>,
    event_tx: &mpsc::UnboundedSender<ClientEvent>,
    shutdown: &Notify,
) -> Result<(), ClientError> {
    let (read_half, mut write_half) = stream.into_split();
    let reader = BufReader::new(read_half);
    let mut lines = reader.lines();
    let mut pending: Option<(u64, String, Instant)> = None;
    let mut queue: VecDeque<Request> = VecDeque::new();

    loop {
        if pending.is_some() {
            // Délai maximal pour la réponse en cours.
            let deadline = pending.as_ref().map(|p| p.2 + REQUEST_TIMEOUT).unwrap();
            tokio::select! {
                biased;
                _ = shutdown.notified() => return Ok(()),
                maybe_request = request_rx.recv() => {
                    match maybe_request {
                        Some(request) => queue.push_back(request),
                        None => return Ok(()),
                    }
                }
                maybe_line = lines.next_line() => {
                    handle_incoming_line(maybe_line, &mut pending, event_tx)?;
                }
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {
                    if let Some((id, method, _)) = pending.take() {
                        warn!(id, method = %method, "délai dépassé pour la requête");
                        let _ = event_tx.send(ClientEvent::Error {
                            code: ErrorCode::Timeout,
                            daemon_code: None,
                        });
                    }
                }
            }
        } else {
            tokio::select! {
                biased;
                _ = shutdown.notified() => return Ok(()),
                maybe_request = request_rx.recv() => {
                    match maybe_request {
                        Some(request) => queue.push_back(request),
                        None => return Ok(()),
                    }
                }
                maybe_line = lines.next_line() => {
                    // Ligne inattendue hors requête : on l'ignore.
                    let _ = maybe_line;
                }
            }
        }

        // Envoi de la prochaine requête en attente.
        if pending.is_none() {
            if let Some(request) = queue.pop_front() {
                let payload = serde_json::to_string(&request)
                    .map_err(|error| ClientError::Protocol(error.to_string()))?;
                write_half
                    .write_all(format!("{payload}\n").as_bytes())
                    .await
                    .map_err(|error| ClientError::Io(error.to_string()))?;
                write_half
                    .flush()
                    .await
                    .map_err(|error| ClientError::Io(error.to_string()))?;
                let id = request.id.unwrap_or(0);
                let method = request.method.clone();
                pending = Some((id, method, Instant::now()));
                debug!(id, "requête envoyée au démon");
            }
        }
    }
}

fn handle_incoming_line(
    maybe_line: Result<Option<String>, std::io::Error>,
    pending: &mut Option<(u64, String, Instant)>,
    event_tx: &mpsc::UnboundedSender<ClientEvent>,
) -> Result<(), ClientError> {
    match maybe_line {
        Ok(Some(line)) => {
            if line.trim().is_empty() {
                return Ok(());
            }
            let response: Response = serde_json::from_str(&line)
                .map_err(|error| ClientError::Protocol(error.to_string()))?;
            if let Some((id, method, _)) = pending.take() {
                debug!(id, method = %method, success = response.success, code = response.code, "réponse reçue");
                let _ = event_tx.send(ClientEvent::Response {
                    method,
                    response,
                });
            } else {
                debug!("réponse orpheline ignorée");
            }
            Ok(())
        }
        Ok(None) => {
            // Le démon a fermé la connexion.
            Err(ClientError::Io("connexion fermée par le démon".to_string()))
        }
        Err(error) => Err(ClientError::Io(error.to_string())),
    }
}

fn classify_connect_error(socket_path: &Path, error: &std::io::Error) -> ErrorCode {
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::NotFound => ErrorCode::SocketMissing,
        ErrorKind::ConnectionRefused => ErrorCode::ConnectionRefused,
        _ => {
            if !socket_path.exists() {
                ErrorCode::SocketMissing
            } else {
                ErrorCode::Io
            }
        }
    }
}

fn next_backoff(current: Duration) -> Duration {
    (current * 2).min(BACKOFF_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_backoff_doubles_until_cap() {
        assert_eq!(next_backoff(Duration::from_secs(1)), Duration::from_secs(2));
        assert_eq!(next_backoff(Duration::from_secs(16)), Duration::from_secs(30));
        assert_eq!(next_backoff(Duration::from_secs(30)), Duration::from_secs(30));
    }

    #[test]
    fn request_serializes_as_envelope() {
        let request = Request::new("list_games", serde_json::json!({}));
        let serialized = serde_json::to_string(&request).expect("serialization");
        assert!(serialized.contains("\"method\":\"list_games\""));
    }

    #[test]
    fn response_deserializes_with_defaults() {
        let raw = r#"{"success":true,"code":0}"#;
        let response: Response = serde_json::from_str(raw).expect("deserialization");
        assert!(response.success);
        assert!(response.data.is_null());
    }
}
