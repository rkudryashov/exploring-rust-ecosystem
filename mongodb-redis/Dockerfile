FROM rust:1.55

ENV CARGO_TERM_COLOR always

WORKDIR /usr/src/app/docker-build
COPY /Cargo.lock ./
COPY /mongodb-redis/ ./
RUN cargo install --path . --locked

FROM debian:buster-slim
COPY --from=0 /usr/local/cargo/bin/mongodb-redis /usr/local/bin/mongodb-redis
CMD ["mongodb-redis"]
