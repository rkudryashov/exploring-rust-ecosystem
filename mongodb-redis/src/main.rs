use std::env;
use std::sync::Arc;

use actix_web::{web, App, HttpServer};
use log::info;

use crate::db::MongoDbClient;
use crate::services::{PlanetService, RateLimitingService};

mod db;
mod dto;
mod errors;
mod handlers;
mod model;
mod redis;
mod services;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::from_filename(".env.local").ok();
    env_logger::init();

    info!("Starting MongoDB & Redis demo server");

    let mongodb_uri = env::var("MONGODB_URI").expect("MONGODB_URI env var should be specified");
    let mongodb_client = MongoDbClient::new(mongodb_uri).await;

    let redis_uri = env::var("REDIS_URI").expect("REDIS_URI env var should be specified");
    let redis_client = redis::create_client(redis_uri)
        .await
        .expect("Can't create Redis client");
    let redis_connection_manager = redis_client
        .get_tokio_connection_manager()
        .await
        .expect("Can't create Redis connection manager");

    let planet_service = Arc::new(PlanetService::new(
        mongodb_client,
        redis_client,
        redis_connection_manager.clone(),
    ));
    let rate_limiting_service = Arc::new(RateLimitingService::new(redis_connection_manager));

    let enable_write_handlers = env::var("ENABLE_WRITE_HANDLERS")
        .expect("MONGODB_URI env var should be specified")
        .parse::<bool>()
        .expect("Can't parse ENABLE_WRITE_HANDLERS");

    HttpServer::new(move || {
        let mut app = App::new()
            .route("/planets", web::get().to(handlers::get_planets))
            .route("/planets/{planet_id}", web::get().to(handlers::get_planet))
            .route(
                "/planets/{planet_id}/image",
                web::get().to(handlers::get_image_of_planet),
            )
            .route("/events", web::get().to(handlers::sse))
            .route("/", web::get().to(handlers::index))
            .data(Arc::clone(&planet_service))
            .data(Arc::clone(&rate_limiting_service));

        if enable_write_handlers {
            app = app
                .route("/planets", web::post().to(handlers::create_planet))
                .route(
                    "/planets/{planet_id}",
                    web::put().to(handlers::update_planet),
                )
                .route(
                    "/planets/{planet_id}",
                    web::delete().to(handlers::delete_planet),
                );
        }

        app
    })
    .bind("0.0.0.0:9000")?
    .run()
    .await
}
