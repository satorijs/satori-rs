use satori::{AppT, BotId, CallApiError, Login, Satori, SdkT};
use serde_json::Value;
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};
use tracing_subscriber::filter::LevelFilter;

pub struct Echo {}

#[async_trait::async_trait]
impl SdkT for Echo {
    type Config = ();
    async fn start<S, A>(&self, _s: &Arc<Satori<S, A>>, _config: Self::Config)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
    }
    async fn shutdown<S, A>(&self, _s: &Arc<Satori<S, A>>)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
    }
    async fn call_api(
        &self,
        _api: &str,
        _bot: &BotId,
        _data: Value,
    ) -> Result<String, CallApiError> {
        Err(CallApiError::ServerError(500))
    }
    async fn get_logins(&self) -> Vec<Login> {
        vec![]
    }
}

#[tokio::main]
async fn main() {
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(LevelFilter::INFO)
        .with_targets([("Satori", LevelFilter::TRACE)]);
    use tracing_subscriber::{
        prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
    };
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter))
        .init();
    let sdk = Satori::new_sdk(Echo {});
    sdk.start(
        (),
        vec![satori::NetAPPConfig {
            host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 5141,
            authorize: None,
        }],
    )
    .await;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await
    }
}
