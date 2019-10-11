FROM vanessa/rustarok-base

# docker build -t vanessa/rustarok .

ARG RUSTAROK_SLEEPMS=0
WORKDIR /code
ADD . /code
RUN sed -i "/sleep_ms = 10/c\sleep_ms = ${RUSTAROK_SLEEPMS}" /code/config-runtime.toml && \
    cat /code/config-runtime.toml | grep sleep_ms && \
    cp /code/docker/config.toml /code/config.toml && \
    cargo build --release
ENTRYPOINT ["/code/target/release/rustarok"]
