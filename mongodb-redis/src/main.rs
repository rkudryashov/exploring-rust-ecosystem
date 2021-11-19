use std::env;

use actix_web::dev::Service;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use log::info;

use crate::broadcaster::Broadcaster;
use crate::db::MongoDbClient;
use crate::services::{PlanetService, RateLimitingService};
use prometheus::HistogramTimer;

mod broadcaster;
mod db;
mod dto;
mod errors;
mod handlers;
mod metrics;
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

    let broadcaster = Broadcaster::create();

    redis::start_pubsub(&redis_client, broadcaster.clone())
        .await
        .expect("Can't start Redis Pub/Sub");

    let planet_service = Data::new(PlanetService::new(
        mongodb_client,
        redis_client,
        redis_connection_manager.clone(),
    ));

    let rate_limiting_service = Data::new(RateLimitingService::new(redis_connection_manager));

    let enable_writing_handlers = env::var("ENABLE_WRITING_HANDLERS")
        .expect("ENABLE_WRITING_HANDLERS env var should be specified")
        .parse::<bool>()
        .expect("Can't parse ENABLE_WRITING_HANDLERS");

    HttpServer::new(move || {
        let mut app = App::new()
            .wrap_fn(|req, srv| {
                let mut histogram_timer: Option<HistogramTimer> = None;
                let request_path = req.path();
                let is_registered_resource = req.resource_map().has_resource(request_path);
                // this check prevents possible DoS attacks that can be done by flooding the application
                // using requests to different unregistered paths. That can cause high memory consumption
                // of the application and Prometheus server and also overflow Prometheus's TSDB
                if is_registered_resource {
                    let request_method = req.method().to_string();
                    histogram_timer = Some(
                        metrics::HTTP_RESPONSE_TIME_SECONDS
                            .with_label_values(&[&request_method, request_path])
                            .start_timer(),
                    );
                    metrics::HTTP_REQUESTS_TOTAL
                        .with_label_values(&[&request_method, request_path])
                        .inc();
                }

                let fut = srv.call(req);

                async {
                    let res = fut.await?;
                    if let Some(histogram_timer) = histogram_timer {
                        histogram_timer.observe_duration();
                    };
                    Ok(res)
                }
            })
            .route("/planets", web::get().to(handlers::get_planets))
            .route("/planets/{planet_id}", web::get().to(handlers::get_planet))
            .route(
                "/planets/{planet_id}/image",
                web::get().to(handlers::get_image_of_planet),
            )
            .route("/events", web::get().to(handlers::sse))
            .route("/", web::get().to(handlers::index))
            .route("/metrics", web::get().to(handlers::metrics))
            .app_data(planet_service.clone())
            .app_data(rate_limiting_service.clone())
            .app_data(broadcaster.clone());

        if enable_writing_handlers {
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
