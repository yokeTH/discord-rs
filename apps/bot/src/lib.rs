pub mod command;
pub mod config;

pub struct Data {}

pub type Error = anyhow::Error;
pub type Context<'a> = poise::Context<'a, Data, Error>;
