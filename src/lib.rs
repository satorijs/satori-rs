use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;

mod net;
pub use net::{NetAPPConfig, NetSDKConfig};
mod structs;
pub use structs::*;

pub struct Satori<S, A> {
    s: S,
    a: A,
}

#[derive(PartialEq, Eq, Hash)]
pub struct BotId {
    pub id: String,
    pub platform: String,
}

pub enum CallApiError {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    ServerError(u16),

    DeserializeFailed,
}

#[async_trait]
pub trait SdkT {
    type Config;
    async fn start<S, A>(&self, s: &Arc<Satori<S, A>>, config: Self::Config)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static;
    async fn shutdown<S, A>(&self, s: &Arc<Satori<S, A>>)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static;
    async fn call_api(&self, api: &str, bot: &BotId, data: Value) -> Result<String, CallApiError>;
    async fn get_logins(&self) -> Vec<Login>;
}

#[async_trait]
pub trait AppT {
    type Config;
    async fn start<S, A>(&self, s: &Arc<Satori<S, A>>, config: Self::Config)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static;
    async fn shutdown<S, A>(&self, s: &Arc<Satori<S, A>>)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static;
    async fn handle_event<S, A>(&self, s: &Arc<Satori<S, A>>, event: Event)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static;
}

impl<S, A> Satori<S, A>
where
    S: SdkT + Send + Sync + 'static,
    A: AppT + Send + Sync + 'static,
{
    pub async fn start(self: &Arc<Self>, sdk_config: S::Config, app_config: A::Config) {
        self.s.start(self, sdk_config).await;
        self.a.start(self, app_config).await;
    }
    pub async fn call_api<T: DeserializeOwned>(
        &self,
        api: &str,
        bot: &BotId,
        data: Value,
    ) -> Result<T, CallApiError> {
        self.s
            .call_api(api, bot, data)
            .await
            .and_then(|s| serde_json::from_str(&s).map_err(|_| CallApiError::DeserializeFailed))
    }
    pub async fn handle_event(self: &Arc<Self>, event: Event) {
        self.a.handle_event(self, event).await
    }
}

pub type SatoriApp<A> = Satori<net::NetSDK, A>;

pub type SatoriSDK<S> = Satori<S, net::NetApp>;

impl<A> SatoriApp<A> {
    pub fn new_app(app: A) -> Arc<Self> {
        Arc::new(Self {
            s: net::NetSDK::default(),
            a: app,
        })
    }
}

impl<S> SatoriSDK<S> {
    pub fn new_sdk(sdk: S) -> Arc<Self> {
        Arc::new(Self {
            s: sdk,
            a: net::NetApp::new(),
        })
    }
}
