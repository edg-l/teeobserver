use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    response::IntoResponse,
    TypedHeader,
};
use serde_json::json;
use time::{OffsetDateTime, format_description};
use tokio::sync::broadcast;
use tracing::{error, info};

use crate::{structures::MasterEvent, AppState};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    info!("`websocket: {user_agent}` at {addr} connected.");

    let rx = state.events_sender.subscribe();
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_failed_upgrade(|error| {
        error!("error upgrading connection: {error}");
    })
    .on_upgrade(move |socket| handle_socket(socket, addr, rx))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(
    mut socket: WebSocket,
    who: SocketAddr,
    mut events_rx: broadcast::Receiver<Arc<(MasterEvent, OffsetDateTime)>>,
) {
    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                if let Ok(msg) = msg {
                    if let Message::Close(_) = msg {
                        info!("client {who} closed connection");
                        return;
                    }
                } else {
                    info!("client {who} abruptly disconnected");
                    return;
                }
            },
            Ok(event) = events_rx.recv() => {
                if let Err(e) = handle_event(&mut socket, event).await {
                    error!("error sending event: {e}");
                }
            }
        }
    }
}

async fn handle_event(
    sock: &mut WebSocket,
    event: Arc<(MasterEvent, OffsetDateTime)>,
) -> Result<(), axum::Error> {
    let payload = json!({
        "time": event.1.format(&format_description::well_known::Iso8601::DEFAULT).unwrap(),
        "event": event.0
    });

    sock.send(Message::Text(payload.to_string())).await
}
