use satori::{AppT, Event, Satori, SdkT};
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};
use tracing_subscriber::filter::LevelFilter;

pub struct Echo {}

#[async_trait::async_trait]
impl AppT for Echo {
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
    async fn handle_event<S, A>(&self, _s: &Arc<Satori<S, A>>, event: Event)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
        println!("{:?}", event)
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
    let app = Satori::new_app(Echo {});
    app.start(
        vec![satori::NetSDKConfig {
            host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 5140,
            authorize: None,
        }],
        (),
    )
    .await;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await
    }
}
