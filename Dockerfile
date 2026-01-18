FROM rust:1.92-bullseye AS builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/jvs-together-websocket /usr/local/bin/myapp
EXPOSE 9001
CMD ["myapp"]
