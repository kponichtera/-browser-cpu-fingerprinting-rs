mod config;
mod context;
mod handlers;
mod repository;

use crate::config::read_config;
use crate::context::build_context;
use crate::handlers::register_handlers;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{middleware, web, App, HttpServer};
use env_logger::Env;
use log::info;
use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(Env::default().default_filter_or("debug"));

    let config = read_config(None);
    let context = build_context(&config).await?;

    let bind_addr = ("0.0.0.0", config.port);
    info!("Starting server on {}:{}", bind_addr.0, bind_addr.1);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(middleware::Compress::default())
            .app_data(Data::new(context.clone()))
            .configure(configure_server)
            .configure(register_handlers)
    })
    .bind(bind_addr)?
    .run()
    .await?;

    Ok(())
}

fn configure_server(cfg: &mut web::ServiceConfig) {
    // Increase the size limit of the payload to 100 MB
    let json_cfg = web::JsonConfig::default().limit(100 * 1024 * 1024);
    let payload_cfg = web::PayloadConfig::default().limit(100 * 1024 * 1024);

    cfg.app_data(json_cfg).app_data(payload_cfg);
}
