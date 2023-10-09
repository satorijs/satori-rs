use super::Signal;
use crate::{AppT, BotId, Event, Satori, SdkT, SATORI};

use async_trait::async_trait;
use axum::extract::{Path, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::Json;
use futures_util::StreamExt;
use hyper::{HeaderMap, StatusCode};
use serde_json::Value;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tracing::{error, info};

pub struct NetApp {
    tx: tokio::sync::broadcast::Sender<Event>,
    stx: tokio::sync::broadcast::Sender<()>,
}

impl NetApp {
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(128);
        let (stx, _) = tokio::sync::broadcast::channel(128);
        Self { tx, stx }
    }
}

#[derive(Clone)]
pub struct NetAPPConfig {
    pub host: IpAddr,
    pub port: u16,
    pub authorize: Option<String>,
}

#[async_trait]
impl AppT for NetApp {
    type Config = Vec<NetAPPConfig>;
    async fn start<S, A>(&self, s: &Arc<Satori<S, A>>, config: Self::Config)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
        for net in config {
            let tx = self.tx.clone();
            let stx = self.stx.clone();
            let s = s.clone();
            tokio::spawn(async move {
                let mut srx = stx.subscribe();
                let app = axum::Router::new()
                    .route(
                        "/v1/events",
                        axum::routing::get({
                            let s = s.clone();
                            move |ws| ws_handle(ws, tx, stx, s)
                        }),
                    )
                    .route(
                        "/v1/:api",
                        axum::routing::post(move |path, map, data| api_handle(path, map, data, s)),
                    );
                let server = axum::Server::bind(&(net.host, net.port).into())
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>());
                info!(target: SATORI, "Start server in {}:{}", net.host, net.port);
                loop {
                    tokio::select! {
                        _ = server => return,
                        Ok(_) = srx.recv() => return,
                    }
                }
            });
        }
    }
    async fn shutdown<S, A>(&self, _s: &Arc<Satori<S, A>>)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
        self.stx.send(()).ok();
    }
    async fn handle_event<S, A>(&self, _s: &Arc<Satori<S, A>>, event: Event)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
        self.tx.send(event).ok();
    }
}

async fn ws_handle<S, A>(
    ws: WebSocketUpgrade,
    tx: tokio::sync::broadcast::Sender<Event>,
    stx: tokio::sync::broadcast::Sender<()>,
    s: Arc<Satori<S, A>>,
) -> impl IntoResponse
where
    S: SdkT + Send + Sync + 'static,
    A: AppT + Send + Sync + 'static,
{
    let mut rx = tx.subscribe();
    let mut srx = stx.subscribe();
    ws.on_upgrade(move |mut socket| async move {
        info!(target: SATORI, "new WebSocket client acceptted.");
        loop {
            tokio::select! {
                Ok(event) = rx.recv() => {
                    if let Err(e) = socket
                        .send(serde_json::to_string(&Signal::event(event)).unwrap().into())
                        .await
                    {
                        error!(target: SATORI, "Send event error: {e}");
                        return;
                    }
                }
                Some(Ok(msg)) = socket.next() => {
                    match msg {
                        axum::extract::ws::Message::Close(_) => return,
                        axum::extract::ws::Message::Ping(b) => {
                            socket.send(axum::extract::ws::Message::Pong(b)).await.ok();
                        }
                        axum::extract::ws::Message::Text(text) => match serde_json::from_str(&text) {
                            Ok(Signal::<Value> { op, body }) => match op {
                                1 => socket
                                    .send(Signal::pong().to_string().into())
                                    .await
                                    .unwrap(), //todo
                                3 => {
                                    let _body = body;
                                    socket
                                        .send(Signal::ready(s.s.get_logins().await).to_string().into())
                                        .await
                                        .unwrap();
                                }
                                _ => unreachable!(),
                            },
                            Err(e) => {
                                error!(target: SATORI, "Receive signal error: {e}")
                            }
                        },
                        _ => {}
                    }
                }
                Ok(_) = srx.recv() => return,
            }
        }
    })
}

async fn api_handle<S, A>(
    Path(api): Path<String>,
    headers: HeaderMap,
    Json(data): Json<Value>,
    s: Arc<Satori<S, A>>,
) -> Result<String, (StatusCode, String)>
where
    S: SdkT + Send + Sync + 'static,
    A: AppT + Send + Sync + 'static,
{
    let Some(id) = headers
        .get("X-Self-ID")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned())
    else {
        return Err((
            StatusCode::BAD_REQUEST,
            "Self id missed or error".to_owned(),
        ));
    };
    let Some(platform) = headers
        .get("X-Platform")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned())
    else {
        return Err((
            StatusCode::BAD_REQUEST,
            "Platform missed or error".to_owned(),
        ));
    };
    match s.call_api(&api, &BotId { platform, id }, data).await {
        Ok(s) => Ok(s),
        Err(e) => Err(e.into_resp()),
    }
}
