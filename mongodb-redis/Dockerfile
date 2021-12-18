FROM rust:1.57

ENV CARGO_TERM_COLOR always

# create empty project for caching dependencies
RUN USER=root cargo new --bin /mongodb-redis/docker-build
WORKDIR /mongodb-redis/docker-build
COPY /Cargo.lock ./
COPY /mongodb-redis/Cargo.toml ./
# cache dependencies
RUN cargo install --path . --locked
COPY /mongodb-redis/ ./
RUN touch ./src/main.rs
RUN cargo install --path . --locked

FROM debian:buster-slim
COPY --from=0 /usr/local/cargo/bin/mongodb-redis /usr/local/bin/mongodb-redis
CMD ["mongodb-redis"]
