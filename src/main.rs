use std::{io::IsTerminal, sync::Arc, time::Duration};

use anyhow::Result;
use bot::Error;
use bot::component::handle_component;
use bot::{Data, command::stock::stock_command, config::Config};
use chrono_tz::America::New_York;
use poise::serenity_prelude as serenity;
use poise::{Framework, FrameworkOptions};
use serenity::all::{ActivityData, ClientBuilder, GatewayIntents};
use stock::{PriceClient, SymbolStore};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info, instrument, warn};
use tracing_futures::Instrument;
use tracing_subscriber::{EnvFilter, fmt};
use webull::WebullClient;

mod daily;
mod token_refresh;

pub async fn event_handler(
    serenity_ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework_ctx: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let serenity::FullEvent::InteractionCreate { interaction, .. } = event
        && let serenity::Interaction::Component(component) = interaction
    {
        debug!(
            custom_id = %component.data.custom_id,
            user_id = %component.user.id,
            "component interaction"
        );

        if let Err(e) = handle_component(serenity_ctx, data, component).await {
            warn!(error = ?e, "handle_component failed");
        }
    }
    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match &error {
        poise::FrameworkError::Command { error: e, ctx, .. } => {
            error!(
                command = %ctx.command().qualified_name,
                user_id = %ctx.author().id,
                error = ?e,
                "command returned an error"
            );
        }
        e => error!(error = %e, "framework error"),
    }

    if let Err(e) = poise::builtins::on_error(error).await {
        error!(error = ?e, "failed to run default poise error handler");
    }
}

#[tokio::main]
#[instrument(name = "main", skip_all)]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Only emit ANSI colors when stdout is a real terminal (and NO_COLOR is
    // unset); otherwise log sinks show raw escape codes.
    let ansi = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_line_number(true)
        .with_ansi(ansi)
        .compact()
        .init();

    let config = Config::from_env();
    info!(version = %config.version, "config loaded");

    let symbol_store = Arc::new(SymbolStore::from_env().await?);
    info!("symbol store initialized");

    let price_client = Arc::new(PriceClient::from_env()?);
    info!("price client initialized");

    // Keep the Webull access token alive in the background (if configured).
    match WebullClient::from_env() {
        Ok(webull) if webull.access_token().is_some() => {
            token_refresh::spawn(Arc::new(webull));
            info!("webull token auto-refresh started");
        }
        Ok(_) => warn!(
            "webull configured but WEBULL_ACCESS_TOKEN is unset; \
             run the create_token example to issue one"
        ),
        Err(e) => debug!(error = ?e, "webull not configured; token refresh disabled"),
    }

    let intents = GatewayIntents::non_privileged();
    let commands = vec![stock_command()];

    let framework = Framework::builder()
        .options(FrameworkOptions {
            event_handler: |ctx, event, fw, data| Box::pin(event_handler(ctx, event, fw, data)),
            commands,
            on_error: |error| Box::pin(on_error(error)),
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
                        bot_user = %ready.user.name,
                        bot_id = %ready.user.id,
                        "connected"
                    );

                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    info!("registered commands globally");

                    // Status: "vX.X.X ∙ HH:MM (+07:00)"
                    let ctx_clone = ctx.clone();
                    tokio::spawn(async move {
                        let mut tick = tokio::time::interval(Duration::from_secs(30));

                        loop {
                            tick.tick().await;

                            let now = chrono::Utc::now().with_timezone(&chrono_tz::Asia::Bangkok);
                            let text =
                                format!("{} ∙ {}", config.version, now.format("%H:%M (%:z)"));

                            ctx_clone.set_activity(Some(ActivityData::custom(text)));
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

    let sched = JobScheduler::new().await?;
    info!("job scheduler created");

    let price_client_job = Arc::clone(&price_client);
    let symbol_store_job = Arc::clone(&symbol_store);

    sched
        .add(Job::new_async_tz(
            "0 30 16 * * Mon-Fri",
            New_York,
            move |_uuid, _l| {
                let http = http.clone();
                let price_client = Arc::clone(&price_client_job);
                let symbol_store = Arc::clone(&symbol_store_job);

                let span = tracing::info_span!("daily_job");
                Box::pin(
                    async move {
                        info!("starting daily run");
                        if let Err(e) =
                            daily::run_daily(http, price_client, symbol_store).await
                        {
                            error!(error = ?e, "run_daily failed");
                        } else {
                            info!("daily run complete");
                        }
                    }
                    .instrument(span),
                )
            },
        )?)
        .await?;
    info!("daily job registered");

    sched.shutdown_on_ctrl_c();
    sched.start().await?;
    info!("job scheduler started");

    tokio::spawn(async move {
        if let Err(why) = client.start().await {
            error!(error = ?why, "discord client error");
        }
    });

    shutdown_signal().await;
    info!("shutdown signal received");

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
