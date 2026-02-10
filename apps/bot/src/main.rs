use std::{sync::Arc, time::Duration};

use anyhow::Result;
use bot::{
    Data,
    command::{self, stock::stock_command},
    config::Config,
};
use chrono_tz::America::New_York;
use log::info;
use poise::{Framework, FrameworkOptions};
use serenity::all::{ActivityData, ClientBuilder, FullEvent, GatewayIntents, Interaction};
use stock::{PriceClient, SymbolStore};
use tokio_cron_scheduler::{Job, JobScheduler};

mod daily;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let config = Config::from_env();

    let symbol_store = Arc::new(
        SymbolStore::from_env()
            .await
            .expect("init symbol store failed"),
    );
    let price_client = Arc::new(PriceClient::from_env().expect("init price client failed"));

    let intents = GatewayIntents::non_privileged();
    let commands = vec![stock_command()];

    let framework = Framework::builder()
        .options(FrameworkOptions {
            event_handler: |serenity_ctx, event, _framework_ctx, data| {
                Box::pin(async move {
                    if let FullEvent::InteractionCreate { interaction, .. } = event
                        && let Interaction::Component(component) = interaction
                    {
                        let _ =
                            command::stock::handle_component(serenity_ctx, data, component).await;
                    }
                    Ok(())
                })
            },
            commands,
            ..Default::default()
        })
        .setup({
            let symbol_store = Arc::clone(&symbol_store);
            let price_client = Arc::clone(&price_client);
            let config = config.clone();

            move |ctx, ready, framework| {
                let symbol_store = Arc::clone(&symbol_store);
                let price_client = Arc::clone(&price_client);
                let config = config.clone();

                Box::pin(async move {
                    info!(
                        "{} [{}] connected successfully!",
                        ready.user.name, ready.user.id
                    );

                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                    let ctx_clone = ctx.clone();
                    tokio::spawn(async move {
                        let mut show_version = true;
                        let mut tick = tokio::time::interval(Duration::from_secs(30));

                        loop {
                            tick.tick().await;

                            let text = if show_version {
                                if config.version.starts_with('v') {
                                    config.version.clone()
                                } else {
                                    format!("Version - {}", config.version)
                                }
                            } else {
                                let now = chrono::Local::now();
                                format!("Time - {}", now.format("%H:%M (%:z)"))
                            };

                            ctx_clone.set_activity(Some(ActivityData::custom(text)));
                            show_version = !show_version;
                        }
                    });

                    Ok(Data {
                        symbol_store,
                        price_client,
                    })
                })
            }
        })
        .build();

    let mut client = ClientBuilder::new(&config.discord_token, intents)
        .framework(framework)
        .await
        .expect("Err creating client");

    let http = client.http.clone();
    let channel_id: u64 = std::env::var("DISCORD_TARGET_CHANNEL_ID")?.parse()?;
    let channel = serenity::all::ChannelId::new(channel_id);

    let sched = JobScheduler::new()
        .await
        .expect("failed to create job scheduler");

    let price_client_job = Arc::clone(&price_client);
    let symbol_store_job = Arc::clone(&symbol_store);

    sched
        .add(Job::new_async_tz(
            "0 30 16 * * Mon-Fri",
            New_York,
            move |_uuid, _l| {
                let http = http.clone();
                let channel = channel;
                let price_client = Arc::clone(&price_client_job);
                let symbol_store = Arc::clone(&symbol_store_job);

                Box::pin(async move {
                    if let Err(e) =
                        daily::run_daily(http, channel, price_client, symbol_store).await
                    {
                        log::error!("run_daily failed: {:?}", e);
                    }
                })
            },
        )?)
        .await?;

    sched.shutdown_on_ctrl_c();
    sched.start().await?;

    tokio::spawn(async move {
        if let Err(why) = client.start().await {
            log::error!("Client error: {why:?}");
        }
    });

    shutdown_signal().await;

    info!("Shutdown complete.");
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::{
            select,
            signal::unix::{SignalKind, signal},
        };
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
        select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv()  => {},
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
