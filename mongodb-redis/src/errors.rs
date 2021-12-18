use actix_web::error::ResponseError;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, HttpResponseBuilder};
use derive_more::{Display, Error};
use log::error;
use redis::RedisError;
use serde::Serialize;

#[derive(Debug, Display, Error)]
pub enum CustomError {
    #[display(fmt = message)]
    MongoDbError {
        message: String,
    },
    #[display(fmt = message)]
    RedisError {
        message: String,
    },
    #[display(fmt = message)]
    NotFound {
        message: String,
    },
    InternalError,
    #[display(
        fmt = "Actual requests count: {}. Permitted requests count: {}",
        actual_count,
        permitted_count
    )]
    TooManyRequests {
        actual_count: u64,
        permitted_count: u64,
    },
}

impl CustomError {
    fn name(&self) -> String {
        let name = match self {
            Self::MongoDbError { message: _ } => "MongoDB error",
            Self::RedisError { message: _ } => "Redis error",
            Self::NotFound { message: _ } => "Resource not found",
            Self::InternalError => "Internal error",
            Self::TooManyRequests {
                actual_count: _,
                permitted_count: _,
            } => "Too many requests",
        };

        String::from(name)
    }
}

impl ResponseError for CustomError {
    fn status_code(&self) -> StatusCode {
        match *self {
            CustomError::MongoDbError { message: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::RedisError { message: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::NotFound { message: _ } => StatusCode::NOT_FOUND,
            CustomError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::TooManyRequests {
                actual_count: _,
                permitted_count: _,
            } => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    fn error_response(&self) -> HttpResponse {
        error!("Error: {}", self.to_string());

        let error_response = ErrorResponse {
            error: self.name(),
            message: self.to_string(),
        };

        HttpResponseBuilder::new(self.status_code())
            .content_type(ContentType::json())
            .body(serde_json::to_string(&error_response).expect("Can't serialize error response"))
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl From<mongodb::error::Error> for CustomError {
    fn from(source: mongodb::error::Error) -> Self {
        Self::MongoDbError {
            message: source.to_string(),
        }
    }
}

impl From<mongodb::bson::de::Error> for CustomError {
    fn from(source: mongodb::bson::de::Error) -> Self {
        Self::MongoDbError {
            message: source.to_string(),
        }
    }
}

impl From<mongodb::bson::ser::Error> for CustomError {
    fn from(source: mongodb::bson::ser::Error) -> Self {
        Self::MongoDbError {
            message: source.to_string(),
        }
    }
}

impl From<mongodb::bson::oid::Error> for CustomError {
    fn from(source: mongodb::bson::oid::Error) -> Self {
        Self::NotFound {
            message: source.to_string(),
        }
    }
}

impl From<RedisError> for CustomError {
    fn from(source: RedisError) -> Self {
        Self::RedisError {
            message: source.to_string(),
        }
    }
}

impl From<serde_json::Error> for CustomError {
    fn from(_source: serde_json::Error) -> Self {
        Self::InternalError
    }
}
