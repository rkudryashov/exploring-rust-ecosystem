use std::str::FromStr;

use actix_web::web::Bytes;
use chrono::{Timelike, Utc};
use log::debug;
use mongodb::bson::oid::ObjectId;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client, FromRedisValue, Value};
use tokio::sync::mpsc::{self, Receiver};
use tokio_stream::StreamExt;

use crate::db::MongoDbClient;
use crate::dto::PlanetMessage;
use crate::errors::CustomError;
use crate::errors::CustomError::{RedisError, TooManyRequests};
use crate::model::{Planet, PlanetType};

const PLANET_KEY_PREFIX: &str = "planet";
const IMAGE_KEY_PREFIX: &str = "image";
const RATE_LIMIT_KEY_PREFIX: &str = "rate_limit";
const MAX_REQUESTS_PER_MINUTE: u64 = 10;
const NEW_PLANETS_CHANNEL_NAME: &str = "new_planets";

#[derive(Clone)]
pub struct PlanetService {
    mongodb_client: MongoDbClient,
    redis_client: Client,
    redis_connection_manager: ConnectionManager,
}

impl PlanetService {
    pub fn new(
        mongodb_client: MongoDbClient,
        redis_client: Client,
        redis_connection_manager: ConnectionManager,
    ) -> Self {
        PlanetService {
            mongodb_client,
            redis_client,
            redis_connection_manager,
        }
    }

    pub async fn get_planets(
        &self,
        planet_type: Option<PlanetType>,
    ) -> Result<Vec<Planet>, CustomError> {
        self.mongodb_client.get_planets(planet_type).await
    }

    pub async fn create_planet(&self, planet: Planet) -> Result<Planet, CustomError> {
        let planet = self.mongodb_client.create_planet(planet).await?;
        self.redis_connection_manager
            .clone()
            .publish(
                NEW_PLANETS_CHANNEL_NAME,
                serde_json::to_string(&PlanetMessage::from(&planet))?,
            )
            .await?;
        Ok(planet)
    }

    pub async fn get_planet(&self, planet_id: &str) -> Result<Planet, CustomError> {
        let cache_key = self.get_planet_cache_key(planet_id);
        let mut con = self.redis_client.get_async_connection().await?;

        let cached_planet = con.get(&cache_key).await?;
        match cached_planet {
            Value::Nil => {
                debug!("Use database to retrieve a planet by id: {}", &planet_id);
                let result: Planet = self
                    .mongodb_client
                    .get_planet(ObjectId::from_str(planet_id)?)
                    .await?;

                let _: () = redis::pipe()
                    .atomic()
                    .set(&cache_key, &result)
                    .expire(&cache_key, 60)
                    .query_async(&mut con)
                    .await?;

                Ok(result)
            }
            Value::Data(val) => {
                debug!("Use cache to retrieve a planet by id: {}", planet_id);
                Ok(serde_json::from_slice(&val)?)
            }
            _ => Err(RedisError {
                message: "Unexpected response from Redis".to_string(),
            }),
        }
    }

    pub async fn update_planet(
        &self,
        planet_id: &str,
        planet: Planet,
    ) -> Result<Planet, CustomError> {
        let updated_planet = self
            .mongodb_client
            .update_planet(ObjectId::from_str(planet_id)?, planet)
            .await?;

        let cache_key = self.get_planet_cache_key(planet_id);
        self.redis_connection_manager.clone().del(cache_key).await?;

        Ok(updated_planet)
    }

    pub async fn delete_planet(&self, planet_id: &str) -> Result<(), CustomError> {
        self.mongodb_client
            .delete_planet(ObjectId::from_str(planet_id)?)
            .await?;

        let cache_key = self.get_planet_cache_key(planet_id);
        self.redis_connection_manager.clone().del(cache_key).await?;

        Ok(())
    }

    pub async fn get_image_of_planet(&self, planet_id: &str) -> Result<Vec<u8>, CustomError> {
        let cache_key = self.get_image_cache_key(planet_id);
        let mut redis_connection_manager = self.redis_connection_manager.clone();

        let cached_image = redis_connection_manager.get(&cache_key).await?;
        match cached_image {
            Value::Nil => {
                debug!(
                    "Use database to retrieve an image of a planet by id: {}",
                    &planet_id
                );
                let planet = self
                    .mongodb_client
                    .get_planet(ObjectId::from_str(planet_id)?)
                    .await?;
                let result = crate::db::get_image_of_planet(&planet.name).await;

                let _: () = redis::pipe()
                    .atomic()
                    .set(&cache_key, result.clone())
                    .expire(&cache_key, 60)
                    .query_async(&mut redis_connection_manager)
                    .await?;

                Ok(result)
            }
            Value::Data(val) => {
                debug!(
                    "Use cache to retrieve an image of a planet by id: {}",
                    &planet_id
                );
                Ok(val)
            }
            _ => Err(RedisError {
                message: "Unexpected response from Redis".to_string(),
            }),
        }
    }

    pub async fn get_new_planets_stream(
        &self,
    ) -> Result<Receiver<Result<Bytes, CustomError>>, CustomError> {
        let (tx, rx) = mpsc::channel::<Result<Bytes, CustomError>>(100);

        tx.send(Ok(Bytes::from("data: Connected\n\n")))
            .await
            .expect("Can't send a message to the stream");

        let mut pubsub_con = self
            .redis_client
            .get_async_connection()
            .await?
            .into_pubsub();
        pubsub_con.subscribe(NEW_PLANETS_CHANNEL_NAME).await?;

        tokio::spawn(async move {
            while let Some(msg) = pubsub_con.on_message().next().await {
                let payload = msg.get_payload().expect("Can't get payload of message");
                let payload: String = FromRedisValue::from_redis_value(&payload)
                    .expect("Can't convert from Redis value");
                let msg = Bytes::from(format!("data: Planet created: {:?}\n\n", payload));
                tx.send(Ok(msg))
                    .await
                    .expect("Can't send a message to the stream");
            }
        });

        Ok(rx)
    }

    fn get_planet_cache_key(&self, planet_id: &str) -> String {
        format!("{}:{}", PLANET_KEY_PREFIX, planet_id)
    }

    fn get_image_cache_key(&self, planet_id: &str) -> String {
        format!("{}:{}:{}", PLANET_KEY_PREFIX, planet_id, IMAGE_KEY_PREFIX)
    }
}

#[derive(Clone)]
pub struct RateLimitingService {
    redis_connection_manager: ConnectionManager,
}

impl RateLimitingService {
    pub fn new(redis_connection_manager: ConnectionManager) -> Self {
        RateLimitingService {
            redis_connection_manager,
        }
    }

    pub async fn assert_rate_limit_not_exceeded(&self, ip_addr: String) -> Result<(), CustomError> {
        let current_minute = Utc::now().minute();
        let rate_limit_key = format!("{}:{}:{}", RATE_LIMIT_KEY_PREFIX, ip_addr, current_minute);

        let (count,): (u64,) = redis::pipe()
            .atomic()
            .incr(&rate_limit_key, 1)
            .expire(&rate_limit_key, 60)
            .query_async(&mut self.redis_connection_manager.clone())
            .await?;

        if count > MAX_REQUESTS_PER_MINUTE {
            Err(TooManyRequests {
                actual_count: count,
                permitted_count: MAX_REQUESTS_PER_MINUTE,
            })
        } else {
            Ok(())
        }
    }
}
