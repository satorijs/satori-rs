use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinHandle;

mod net;
pub use net::{NetAPPConfig, NetSDKConfig};
mod structs;
pub use structs::*;

pub const SATORI: &str = "Satori";

pub struct Satori<S, A> {
    s: S,
    a: A,
    stx: tokio::sync::broadcast::Sender<()>,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct BotId {
    pub id: String,
    pub platform: String,
}

#[derive(Debug)]
pub enum CallApiError {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    ServerError(u16),

    DeserializeFailed(serde_json::Error),
}

#[async_trait]
pub trait SdkT {
    type Config;
    async fn start<S, A>(&self, s: &Arc<Satori<S, A>>, config: Self::Config) -> Vec<JoinHandle<()>>
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static;
    async fn call_api(&self, api: &str, bot: &BotId, data: Value) -> Result<String, CallApiError>;
    async fn get_logins(&self) -> Vec<Login>;
    #[allow(unused_variables)]
    async fn on_shutdown<S, A>(&self, s: &Arc<Satori<S, A>>)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
    }
}

#[async_trait]
pub trait AppT {
    type Config;
    async fn start<S, A>(&self, s: &Arc<Satori<S, A>>, config: Self::Config) -> Vec<JoinHandle<()>>
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static;
    async fn handle_event<S, A>(&self, s: &Arc<Satori<S, A>>, event: Event)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static;
    #[allow(unused_variables)]
    async fn on_shutdown<S, A>(&self, s: &Arc<Satori<S, A>>)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
    }
}

impl<S, A> Satori<S, A>
where
    S: SdkT + Send + Sync + 'static,
    A: AppT + Send + Sync + 'static,
{
    pub async fn new(s: S, a: A) -> Arc<Self> {
        Arc::new(Self {
            s,
            a,
            stx: tokio::sync::broadcast::channel(4).0,
        })
    }
    pub async fn start_and_wait(self: &Arc<Self>, sdk_config: S::Config, app_config: A::Config) {
        let mut joins = self.s.start(self, sdk_config).await;
        joins.extend(self.a.start(self, app_config).await);
        for join in joins {
            join.await.ok();
        }
    }
    pub async fn start(self: &Arc<Self>, sdk_config: S::Config, app_config: A::Config) {
        self.s.start(self, sdk_config).await;
        self.a.start(self, app_config).await;
    }
    pub async fn shutdown(self: &Arc<Self>) {
        self.stx.send(()).ok();
        self.s.on_shutdown(self).await;
        self.a.on_shutdown(self).await;
    }
    pub async fn call_api<T: DeserializeOwned>(
        &self,
        api: &str,
        bot: &BotId,
        data: Value,
    ) -> Result<T, CallApiError> {
        self.s.call_api(api, bot, data).await.and_then(|s| {
            tracing::trace!(target:SATORI, "recive api resp:{s}");
            serde_json::from_str(&s).map_err(|e| CallApiError::DeserializeFailed(e))
        })
    }
    pub async fn handle_event(self: &Arc<Self>, event: Event) {
        self.a.handle_event(self, event).await
    }
    pub fn get_stx(&self) -> tokio::sync::broadcast::Sender<()> {
        self.stx.clone()
    }
}

pub type SatoriApp<A> = Satori<net::NetSDK, A>;

pub type SatoriSDK<S> = Satori<S, net::NetApp>;

impl<A> SatoriApp<A> {
    pub fn new_app(app: A) -> Arc<Self> {
        Arc::new(Self {
            s: net::NetSDK::default(),
            a: app,
            stx: tokio::sync::broadcast::channel(4).0,
        })
    }
}

impl<S> SatoriSDK<S> {
    pub fn new_sdk(sdk: S) -> Arc<Self> {
        Arc::new(Self {
            s: sdk,
            a: net::NetApp::new(),
            stx: tokio::sync::broadcast::channel(4).0,
        })
    }
}
