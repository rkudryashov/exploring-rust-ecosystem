use actix_web::http::header::{self, ContentType};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse};
use prometheus::{Encoder, TextEncoder};
use serde::Deserialize;

use crate::broadcaster::Broadcaster;
use crate::dto::PlanetDto;
use crate::errors::CustomError;
use crate::model::PlanetType;
use crate::services::{PlanetService, RateLimitingService};
use std::sync::Mutex;

#[derive(Debug, Deserialize)]
pub struct GetPlanetsQueryParams {
    r#type: Option<PlanetType>,
}

pub async fn get_planets(
    req: HttpRequest,
    web::Query(query_params): web::Query<GetPlanetsQueryParams>,
    rate_limit_service: web::Data<RateLimitingService>,
    planet_service: web::Data<PlanetService>,
) -> Result<HttpResponse, CustomError> {
    // can be moved to actix middleware
    rate_limit_service
        .assert_rate_limit_not_exceeded(get_ip_addr(&req)?)
        .await?;

    let planets = planet_service.get_planets(query_params.r#type).await?;
    Ok(HttpResponse::Ok().json(planets.into_iter().map(PlanetDto::from).collect::<Vec<_>>()))
}

pub async fn create_planet(
    planet_dto: web::Json<PlanetDto>,
    planet_service: web::Data<PlanetService>,
) -> Result<HttpResponse, CustomError> {
    let planet = planet_service
        .create_planet(planet_dto.into_inner().into())
        .await?;

    Ok(HttpResponse::Ok().json(PlanetDto::from(planet)))
}

pub async fn get_planet(
    planet_id: web::Path<String>,
    planet_service: web::Data<PlanetService>,
) -> Result<HttpResponse, CustomError> {
    let planet = planet_service.get_planet(&planet_id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(PlanetDto::from(planet)))
}

pub async fn update_planet(
    planet_id: web::Path<String>,
    planet_dto: web::Json<PlanetDto>,
    planet_service: web::Data<PlanetService>,
) -> Result<HttpResponse, CustomError> {
    let planet = planet_service
        .update_planet(&planet_id.into_inner(), planet_dto.into_inner().into())
        .await?;

    Ok(HttpResponse::Ok().json(PlanetDto::from(planet)))
}

pub async fn delete_planet(
    planet_id: web::Path<String>,
    planet_service: web::Data<PlanetService>,
) -> Result<HttpResponse, CustomError> {
    planet_service
        .delete_planet(&planet_id.into_inner())
        .await?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn get_image_of_planet(
    planet_id: web::Path<String>,
    planet_service: web::Data<PlanetService>,
) -> Result<HttpResponse, CustomError> {
    let image = planet_service
        .get_image_of_planet(&planet_id.into_inner())
        .await?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::png())
        .body(image))
}

pub async fn sse(broadcaster: web::Data<Mutex<Broadcaster>>) -> Result<HttpResponse, CustomError> {
    let rx = broadcaster
        .lock()
        .expect("Can't lock broadcaster")
        .new_client()
        .await;
    let response_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    Ok(HttpResponse::build(StatusCode::OK)
        .insert_header(header::ContentType(mime::TEXT_EVENT_STREAM))
        .streaming(response_stream))
}

pub async fn index() -> Result<HttpResponse, CustomError> {
    let content = include_str!("index.html");

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType(mime::TEXT_HTML))
        .body(content))
}

pub async fn metrics() -> Result<HttpResponse, CustomError> {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .expect("Failed to encode metrics");

    let response = String::from_utf8(buffer.clone()).expect("Failed to convert bytes to string");
    buffer.clear();

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType(mime::TEXT_PLAIN))
        .body(response))
}

fn get_ip_addr(req: &HttpRequest) -> Result<String, CustomError> {
    Ok(req
        .peer_addr()
        .ok_or(CustomError::InternalError)?
        .ip()
        .to_string())
}
