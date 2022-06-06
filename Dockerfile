FROM rust:1.61.0 as builder

WORKDIR /usr/src/bridelopp
COPY . .

RUN cargo install --profile release --path .


FROM debian:buster-slim
WORKDIR /usr/dist/bridelopp
COPY --from=builder /usr/local/cargo/bin/bridelopp /usr/local/bin/bridelopp
COPY --from=builder /usr/src/bridelopp/public /usr/dist/bridelopp/public
COPY --from=builder /usr/src/bridelopp/templates /usr/dist/bridelopp/templates
COPY --from=builder /usr/src/bridelopp/Rocket.toml /usr/dist/bridelopp/Rocket.toml
CMD ["bridelopp"]

