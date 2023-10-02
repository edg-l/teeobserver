use std::{
    collections::{HashSet, VecDeque},
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{routing::get, Router};
use http::Method;
use reqwest::Client;
use sqlx::{postgres::PgPoolOptions, PgPool};
use structures::{MasterEvent, ServerListMap};
use time::{OffsetDateTime, UtcOffset};
use tokio::{
    sync::{
        broadcast::{self, Sender},
        RwLock,
    },
    time::interval,
};
use tower_http::{
    cors::{self, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{error, info};

use crate::util::fetch_master;

pub mod routes;
pub mod structures;
pub mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Better error reporting whe compiling if we are not inside a macro.
    run().await
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub servers: Arc<RwLock<ServerListMap>>,
    pub client: Client,
    pub events: Arc<RwLock<VecDeque<(MasterEvent, OffsetDateTime)>>>,
    pub events_sender: Arc<Sender<Arc<(MasterEvent, OffsetDateTime)>>>,
}

async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let pool = PgPoolOptions::new()
        .max_connections(
            std::env::var("DATABASE_MAX_CONNECTIONS")
                .map(|x| x.parse().expect("valid number"))
                .unwrap_or(30),
        )
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let client = reqwest::Client::new();
    let servers = fetch_master(&client).await?;
    let servers = Arc::new(RwLock::new(servers));
    let events = Arc::new(RwLock::new(VecDeque::new()));

    // let task_pool = pool.clone();
    let task_client = client.clone();
    let task_servers = servers.clone();
    let task_events = events.clone();

    let (events_sender, _) = broadcast::channel::<Arc<(MasterEvent, OffsetDateTime)>>(
        std::env::var("EVENTS_CAPACITY")
            .unwrap_or_else(|_| "10000".to_string())
            .parse()?,
    );
    let events_sender = Arc::new(events_sender);

    let state = AppState {
        pool,
        client,
        servers,
        events,
        events_sender: events_sender.clone(),
    };

    let mut fetch_interval = interval(Duration::from_secs(10));
    let task = tokio::spawn(async move {
        let mut events = Vec::with_capacity(500);
        let empty = vec![];

        loop {
            fetch_interval.tick().await;
            let res = fetch_master(&task_client).await;

            match res {
                Ok(new_servers) => {
                    {
                        events.clear();
                        let old_servers = task_servers.read().await;
                        let now = OffsetDateTime::now_utc();
                        let mut new_players = HashSet::with_capacity(64);
                        let mut old_players = HashSet::with_capacity(64);

                        for (address, server) in new_servers.servers.iter() {
                            if let Some(old_server) = old_servers.servers.get(address) {
                                new_players.clear();
                                old_players.clear();

                                old_players
                                    .extend(old_server.info.clients.as_ref().unwrap_or(&empty));
                                new_players.extend(server.info.clients.as_ref().unwrap_or(&empty));

                                let diff = new_players.symmetric_difference(&old_players);

                                for player in diff {
                                    // client joined
                                    if new_players.contains(player) {
                                        /*
                                        info!(
                                            "'{}' client joined server '{}' ({:?})",
                                            player.name.as_deref().unwrap_or(""),
                                            server.info.name.as_deref().unwrap_or(""),
                                            address
                                        );*/
                                        events.push((
                                            MasterEvent::ClientJoined(
                                                (*player).clone(),
                                                server.clone(),
                                            ),
                                            now,
                                        ));
                                    } else {
                                        /*
                                        info!(
                                            "'{}' client left server '{}' ({:?})",
                                            player.name.as_deref().unwrap_or(""),
                                            server.info.name.as_deref().unwrap_or(""),
                                            address
                                        );
                                        */
                                        events.push((
                                            MasterEvent::ClientLeft(
                                                (*player).clone(),
                                                server.clone(),
                                            ),
                                            now,
                                        ));
                                    }
                                }
                            } else {
                                /*
                                info!(
                                    "server '{}' ({:?}) went online",
                                    server.info.name.as_deref().unwrap_or(""),
                                    address
                                );
                                */
                                events.push((MasterEvent::ServerWentOnline(server.clone()), now));
                            }
                        }

                        for (address, server) in old_servers.servers.iter() {
                            if !new_servers.servers.contains_key(address) {
                                /*
                                info!(
                                    "server '{}' ({:?}) went offline",
                                    server.info.name.as_deref().unwrap_or(""),
                                    address
                                );
                                 */
                                events.push((MasterEvent::ServerWentOffline(server.clone()), now));
                            }
                        }
                    }

                    if events_sender.receiver_count() > 0 {
                        for ev in &events {
                            if let Err(e) = events_sender.send(ev.clone().into()) {
                                error!("error sending event: {e}");
                            }
                        }
                    }

                    let mut lock_events = task_events.write().await;
                    lock_events.extend(events.drain(..));
                    let mut servers = task_servers.write().await;
                    *servers = new_servers;
                }
                Err(e) => error!("error fetching master: {e:?}"),
            }
        }
    });

    let app = Router::new()
        .route("/", get(|| async { "a" }))
        .route("/ws", get(routes::ws_handler))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(5)))
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_origin(cors::Any),
        )
        .with_state(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    task.abort();

    Ok(())
}
