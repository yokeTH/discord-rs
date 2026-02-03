use bot::{Data, command::stock::stock_command, config::Config};
use log::info;
use poise::{Framework, FrameworkOptions};
use serenity::all::{ActivityData, ClientBuilder, GatewayIntents};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    env_logger::init();

    let config = Config::from_env();

    let intents = GatewayIntents::non_privileged();

    let commands = vec![stock_command()];

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands,
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                info!(
                    "{} [{}] connected successfully!",
                    ready.user.name, ready.user.id
                );

                ctx.set_activity(Some(ActivityData::custom(format!("V. {}", config.version))));

                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                Ok(Data {})
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
