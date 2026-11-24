# Build stage
FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Run stage, only the binary
FROM alpine:3
WORKDIR /app
COPY --from=builder /app/target/release/users /app/users
EXPOSE 8080
CMD ["/app/users"]
