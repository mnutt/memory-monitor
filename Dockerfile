FROM rust:1.74

COPY ./ ./

# RUN apt update && apt install -y valgrind

RUN cargo build

ENTRYPOINT ["./target/debug/memory-monitor"]
