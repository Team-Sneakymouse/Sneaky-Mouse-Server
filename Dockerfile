FROM rust:1.56

WORKDIR /sneaky-mouse-server

COPY ./Cargo* ./
# This is needed for rust to start compiling the dependencies
COPY ./src/dummy.rs ./src/main.rs
RUN cargo build --release
RUN rm src/*.rs
RUN rm ./target/release/sneaky-mouse-server

COPY ./src/ ./src/
RUN cargo build --release

CMD ["./target/release/sneaky-mouse-server"]