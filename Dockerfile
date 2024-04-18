# Build Stage 
FROM rust:1.77.0-slim-buster as builder

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

RUN USER=root cargo new --bin tichu-rs
WORKDIR ./tichu-rs
COPY ./Cargo.toml ./Cargo.toml

# Build empty app with downloaded dependencies to produce a stable image layer for next build
RUN cargo build --release

# Build web app with own code
RUN rm src/*.rs
ADD . ./
RUN rm ./target/release/deps/tichu_rs*
RUN cargo build --release

FROM debian:buster-slim
ARG APP=/usr/src/app

RUN apt-get update && apt-get install libssl1.1 -y && rm -rf /var/lib/apt/lists/*

EXPOSE 3000

ENV TZ=Etc/UTC \
    APP_USER=10001

#RUN groupadd $APP_USER \
#    && useradd -g $APP_USER $APP_USER \
#    && mkdir -p ${APP}

COPY --from=builder /tichu-rs/target/release/tichu-rs ${APP}/tichu-rs

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

CMD ["./tichu-rs"]
