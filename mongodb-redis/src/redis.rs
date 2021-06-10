use redis::{Client, RedisError};
use redis::{RedisWrite, ToRedisArgs};

use crate::model::Planet;

pub async fn create_client(redis_uri: String) -> Result<Client, RedisError> {
    Ok(Client::open(redis_uri)?)
}

impl ToRedisArgs for &Planet {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        out.write_arg_fmt(serde_json::to_string(self).expect("Can't serialize Planet as string"))
    }
}
