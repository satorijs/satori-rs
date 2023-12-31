use super::{Logins, Signal};
use crate::{AppT, BotId, CallApiError, Event, Login, Satori, SdkT, SATORI};

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use hyper::{Body, Client, StatusCode};
use serde_json::Value;
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_tungstenite::{
    client_async,
    tungstenite::{handshake::client::generate_key, http::request::Builder, Message},
};
use tracing::{error, info, trace, warn};

#[derive(Default)]
pub struct NetSDK {
    pub bots: Arc<RwLock<HashMap<BotId, NetSDKConfig>>>,
}

#[derive(Clone, Debug)]
pub struct NetSDKConfig {
    pub host: std::net::IpAddr,
    pub port: u16,
    pub authorize: Option<String>,
}

async fn handle_signal<S, A>(
    s: &Arc<Satori<S, A>>,
    signal: Signal<Option<Value>>,
    bots: &Arc<RwLock<HashMap<BotId, NetSDKConfig>>>,
    net: &NetSDKConfig,
    seq: &mut i64,
) where
    S: SdkT + Send + Sync + 'static,
    A: AppT + Send + Sync + 'static,
{
    match signal.op {
        0 => {
            if let Some(body) = signal.body {
                match serde_json::from_value::<Event>(body) {
                    Ok(event) => {
                        info!(target: SATORI, "receive event: {:?}", event);
                        let s = s.clone();
                        *seq = event.id;
                        tokio::spawn(async move { s.handle_event(event).await });
                    }
                    Err(e) => {
                        warn!(target: SATORI, "deserlize event error:{e}");
                    }
                }
            }
        }
        2 => {}
        4 => {
            if let Some(body) = signal.body {
                match serde_json::from_value::<Logins>(body) {
                    Ok(logins) => {
                        let mut bots = bots.write().await;
                        for login in logins.logins {
                            bots.insert(
                                BotId {
                                    platform: login.platform.unwrap(),
                                    id: login.self_id.unwrap(),
                                },
                                net.clone(),
                            );
                        }
                    }
                    Err(e) => {
                        warn!(target: SATORI, "deserlize logins error:{e}")
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}

#[async_trait]
impl SdkT for NetSDK {
    type Config = Vec<NetSDKConfig>;
    async fn start<S, A>(&self, s: &Arc<Satori<S, A>>, config: Self::Config) -> Vec<JoinHandle<()>>
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
        let mut joins = vec![];
        for net in config {
            let mut srx = s.get_stx().subscribe();
            let s = s.clone();
            let bots = self.bots.clone();
            joins.push(tokio::spawn(async move {
                let uri = format!("{}:{}", net.host, net.port);
                let stream = TcpStream::connect(&uri).await.unwrap(); //todo
                let (mut ws_stream, _) = client_async(
                    Builder::new()
                        .method("GET")
                        .header("Host", net.host.clone().to_string())
                        .header("Connection", "Upgrade")
                        .header("Upgrade", "websocket")
                        .header("Sec-WebSocket-Version", "13")
                        .header("Sec-WebSocket-Key", generate_key())
                        .uri(format!("ws://{uri}/v1/events"))
                        .body(())
                        .unwrap(),
                    stream,
                )
                .await
                .unwrap();
                info!(target:SATORI, "WebSocket connected with ws://{uri}/v1/events");

                let mut send_time = tokio::time::Instant::now() + Duration::from_secs(10);
                let mut seq = 0i64;
                ws_stream
                    .send(
                        Signal::identfy(&net.authorize.clone().unwrap_or("".to_string()), seq)
                            .to_string()
                            .into(),
                    )
                    .await
                    .unwrap();
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep_until(send_time) => {
                            ws_stream.send(
                                    Signal::ping().to_string().into()
                                ).await.unwrap();
                            send_time += Duration::from_secs(10);
                        }
                        data = ws_stream.next() => {
                            trace!(target: SATORI, "receive ws_msg: {:?}" ,data);
                            match data {
                                Some(Ok(Message::Text(text))) => match serde_json::from_str(&text) {
                                    Ok(signal) => handle_signal(&s,signal,&bots,&net, &mut seq).await,
                                    Err(e) =>  error!(target: SATORI, "deserialize error: {e} in {text}"),
                                }
                                Some(Ok(Message::Ping(d))) => match ws_stream.send(Message::Pong(d)).await {
                                    Ok(_) => {}
                                    Err(_) => break,
                                }
                                Some(Ok(Message::Pong(_))) => {}
                                _ => break,
                            }
                        }
                        _ = srx.recv() => {
                            ws_stream.send(Message::Close(None)).await.ok();
                            break;
                        }
                    }
                }
            }));
        }
        joins
    }
    async fn call_api(&self, api: &str, bot: &BotId, data: Value) -> Result<String, CallApiError> {
        let mut req = Builder::new()
            .method("POST")
            .header("Content-Type", "application/json")
            .header("X-Platform", &bot.platform)
            .header("X-Self-ID", &bot.id);
        if let Some(net) = self.bots.read().await.get(bot) {
            req = req.uri(format!("http://{}:{}/v1/{}", net.host, net.port, api));
            if let Some(token) = &net.authorize {
                req = req.header("Authorization", format!("Bearer {}", token));
            }
        } else {
            return Err(CallApiError::NotFound);
        }
        let req = req
            .body(Body::from(serde_json::to_string(&data).unwrap()))
            .unwrap();
        trace!(target: SATORI,"Request:{:?}", req);
        let client = Client::new();
        let resp = client.request(req).await.unwrap();
        trace!(target: SATORI,"Response:{:?}", resp);
        match resp.status() {
            StatusCode::OK => {
                let body = hyper::body::to_bytes(resp).await.unwrap();
                Ok(String::from_utf8(body.to_vec()).unwrap())
            }
            StatusCode::NOT_FOUND => Err(CallApiError::NotFound),
            _ => unimplemented!(),
        }
    }
    async fn get_logins(&self) -> Vec<Login> {
        self.bots
            .read()
            .await
            .keys()
            .map(|bot| Login {
                user: None,
                self_id: Some(bot.id.clone()),
                platform: Some(bot.platform.clone()),
                status: crate::Status::Online,
            })
            .collect()
    }
}
