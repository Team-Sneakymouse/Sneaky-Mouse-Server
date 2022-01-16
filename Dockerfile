FROM rust:1.56

WORKDIR /sneaky-mouse-server

COPY ./Cargo* ./
# This is needed for rust to start compiling the dependencies
COPY ./src/dummy.rs ./src/main.rs
RUN cargo build --release
RUN cargo clean -p sneaky-mouse-server --release
RUN rm src/*.rs

COPY ./src/ ./src/
RUN cargo build --release

CMD ["./target/release/sneaky-mouse-server"]
