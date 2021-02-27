FROM rust:1.51

ENV CARGO_TERM_COLOR always
RUN apt-get update && apt-get install -y libpq-dev
RUN rustup component add rustfmt

WORKDIR /usr/src/app/docker-build
COPY /Cargo.lock ./
COPY /solar-system-info/server/ ./
COPY /solar-system-info/rpc ../rpc
RUN cargo install --path . --locked

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libpq-dev
COPY --from=0 /usr/local/cargo/bin/solar-system-info-server /usr/local/bin/solar-system-info-server
CMD ["solar-system-info-server"]
