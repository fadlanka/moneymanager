FROM rust:1-bookworm AS build
WORKDIR /app
# Queries are checked against the committed .sqlx data, so no database is needed to build.
ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY fixtures ./fixtures
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 app \
    && mkdir -p /data/uploads \
    && chown -R app /data
COPY --from=build /app/target/release/budget-backend /usr/local/bin/budget-backend
USER app
ENV BIND_ADDR=0.0.0.0:8080 \
    UPLOAD_DIR=/data/uploads \
    RUST_LOG=info,tower_http=info,sqlx=warn
EXPOSE 8080
CMD ["budget-backend"]
