use actix_web::web::{Bytes, Data};
use redis::{Client, FromRedisValue, RedisError};
use redis::{RedisWrite, ToRedisArgs};
use tokio_stream::StreamExt;

use crate::broadcaster::Broadcaster;
use crate::errors::CustomError;
use crate::model::Planet;
use crate::services::NEW_PLANETS_CHANNEL_NAME;
use std::sync::Mutex;

pub async fn create_client(redis_uri: String) -> Result<Client, RedisError> {
    Ok(Client::open(redis_uri)?)
}

pub async fn start_pubsub(
    redis_client: &Client,
    broadcaster: Data<Mutex<Broadcaster>>,
) -> Result<(), CustomError> {
    let mut pubsub_con = redis_client.get_async_connection().await?.into_pubsub();
    pubsub_con.subscribe(NEW_PLANETS_CHANNEL_NAME).await?;

    tokio::spawn(async move {
        while let Some(msg) = pubsub_con.on_message().next().await {
            let payload = msg.get_payload().expect("Can't get payload of message");
            let payload: String =
                FromRedisValue::from_redis_value(&payload).expect("Can't convert from Redis value");
            let msg = Bytes::from(format!("data: Planet created: {:?}\n\n", payload));
            broadcaster
                .lock()
                .expect("Can't lock broadcaster")
                .send(msg);
        }
    });

    Ok(())
}

impl ToRedisArgs for &Planet {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        out.write_arg_fmt(serde_json::to_string(self).expect("Can't serialize Planet as string"))
    }
}
