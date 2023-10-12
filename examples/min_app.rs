use satori::{AppT, ChannelType, Event, Satori, SdkT, SATORI};
use serde_json::{json, Value};
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};
use tokio::task::JoinHandle;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;

pub struct Echo {}

#[async_trait::async_trait]
impl AppT for Echo {
    type Config = ();
    async fn start<S, A>(
        &self,
        _s: &Arc<Satori<S, A>>,
        _config: Self::Config,
    ) -> Vec<JoinHandle<()>>
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
        vec![]
    }
    async fn handle_event<S, A>(&self, s: &Arc<Satori<S, A>>, mut event: Event)
    where
        S: SdkT + Send + Sync + 'static,
        A: AppT + Send + Sync + 'static,
    {
        info!("start_handle_evnet");
        if let Some(user) = event.user {
            if user.id == event.self_id {
                info!("self event");
                return;
            }
        }
        if let Some(message) = event
            .extra
            .remove("message")
            .and_then(|v| serde_json::from_value::<satori::Message>(v).ok())
            .filter(|m| m.content.starts_with("echo"))
        {
            let bot = satori::BotId {
                id: event.self_id,
                platform: event.platform,
            };
            if let Some(ch) = event.channel {
                match ch.ty {
                    ChannelType::Text => {
                        let r = s
                            .call_api::<Value>(
                                "message.create",
                                &bot,
                                json!({
                                    "channel_id": ch.id,
                                    "content": message.content
                                }),
                            )
                            .await;
                        println!("......r:{:?}", r);
                    }
                    // ChannelType::Direct => {
                    //     let _ch = s
                    //         .call_api::<Channel>(
                    //             "user.channel.create",
                    //             &bot,
                    //             json!({
                    //                 "user_id": ch.id,
                    //             }),
                    //         )
                    //         .await
                    //         .unwrap();
                    // }
                    _ => {}
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(LevelFilter::INFO)
        .with_targets([(SATORI, LevelFilter::TRACE)]);
    use tracing_subscriber::{
        prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
    };
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter))
        .init();
    let app = Satori::new_app(Echo {});
    app.start_and_wait(
        vec![satori::NetSDKConfig {
            host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 5140,
            authorize: None,
        }],
        (),
    )
    .await;
}
