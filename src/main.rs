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
use structures::{MasterEvent, ServerList, ServerListMap};
use tokio::{sync::RwLock, time::interval};
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
    pub events: Arc<RwLock<VecDeque<(MasterEvent, Instant)>>>,
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

    let task_pool = pool.clone();
    let task_client = client.clone();
    let task_servers = servers.clone();
    let task_events = events.clone();

    let state = AppState {
        pool,
        client,
        servers,
        events,
    };

    let mut fetch_interval = interval(Duration::from_secs(10));
    let task = tokio::spawn(async move {
        loop {
            fetch_interval.tick().await;
            let res = fetch_master(&task_client).await;

            match res {
                Ok(new_servers) => {
                    {
                        let old_servers = task_servers.read().await;
                        let mut events = task_events.write().await;
                        let now = Instant::now();
                        let mut new_players = HashSet::with_capacity(64);
                        let mut old_players = HashSet::with_capacity(64);
                        let empty = vec![];

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
                                        info!(
                                            "'{:?}' client joined server '{:?}' ({:?})",
                                            player.name, server.info.name, address
                                        );
                                        events.push_back((
                                            MasterEvent::ClientJoined(
                                                (*player).clone(),
                                                server.clone(),
                                            ),
                                            now,
                                        ));
                                    } else {
                                        info!(
                                            "'{:?}' client left server '{:?}' ({:?})",
                                            player.name, server.info.name, address
                                        );
                                        events.push_back((
                                            MasterEvent::ClientLeft(
                                                (*player).clone(),
                                                server.clone(),
                                            ),
                                            now,
                                        ));
                                    }
                                }
                            } else {
                                info!(
                                    "server '{:?}' ({:?}) went online",
                                    server.info.name, address
                                );
                                events.push_back((
                                    MasterEvent::ServerWentOnline(server.clone()),
                                    now,
                                ));
                            }
                        }

                        for (address, server) in old_servers.servers.iter() {
                            if !new_servers.servers.contains_key(address) {
                                info!(
                                    "server '{:?}' ({:?}) went offline",
                                    server.info.name, address
                                );
                                events.push_back((
                                    MasterEvent::ServerWentOffline(server.clone()),
                                    now,
                                ));
                            }
                        }
                    }

                    let mut servers = task_servers.write().await;
                    *servers = new_servers;
                }
                Err(e) => error!("error fetching master: {e:?}"),
            }
        }
    });

    let app = Router::new()
        .route("/", get(|| async { "a" }))
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
        .serve(app.into_make_service())
        .await?;

    task.abort();

    Ok(())
}
