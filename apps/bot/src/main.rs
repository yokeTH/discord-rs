use std::{sync::Arc, time::Duration};

use anyhow::Result;
use bot::{
    Data,
    command::{self, stock::stock_command},
    config::Config,
};
use log::info;
use poise::{Framework, FrameworkOptions};
use serenity::all::{ActivityData, ClientBuilder, FullEvent, GatewayIntents, Interaction};
use stock::{PriceClient, SymbolStore};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let config = Config::from_env();

    let symbol_store = SymbolStore::from_env()
        .await
        .expect("init price client failed");

    let price_client = PriceClient::from_env().expect("init price client failed");

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
        .setup(move |ctx, ready, framework| {
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
                    symbol_store: Arc::new(symbol_store),
                    price_client: Arc::new(price_client),
                })
            })
        })
        .build();

    let mut client = ClientBuilder::new(&config.discord_token, intents)
        .framework(framework)
        .await
        .expect("Err creating client");

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
