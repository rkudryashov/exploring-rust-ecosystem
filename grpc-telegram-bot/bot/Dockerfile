FROM rust:1.56

ENV CARGO_TERM_COLOR always
RUN rustup component add rustfmt

WORKDIR /usr/src/app/docker-build
COPY /Cargo.lock ./
COPY /grpc-telegram-bot/bot/ ./
COPY /grpc-telegram-bot/rpc ../rpc
RUN cargo install --path . --locked

FROM debian:buster-slim
RUN apt-get update && apt-get install -y openssl
COPY --from=0 /usr/local/cargo/bin/solar-system-info-bot /usr/local/bin/solar-system-info-bot
CMD ["solar-system-info-bot"]
