FROM rust:1.71.0 AS chef
RUN cargo install cargo-chef 
WORKDIR /usr/src/bridelopp


FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /usr/src/bridelopp/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo install --profile release --path .


FROM debian:11 AS runtime

RUN apt update -y
RUN apt install -y ca-certificates
RUN apt install -y openssl
RUN update-ca-certificates

WORKDIR /usr/dist/bridelopp
COPY --from=builder /usr/local/cargo/bin/bridelopp /usr/local/bin/bridelopp
COPY --from=builder /usr/src/bridelopp/public /usr/dist/bridelopp/public
COPY --from=builder /usr/src/bridelopp/templates /usr/dist/bridelopp/templates
COPY --from=builder /usr/src/bridelopp/Rocket.toml /usr/dist/bridelopp/Rocket.toml
CMD ["bridelopp"]

