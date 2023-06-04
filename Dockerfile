FROM rust:1-bullseye AS builder

RUN curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | apt-key add -
RUN curl -fsSL https://deb.nodesource.com/setup_current.x | bash -
RUN echo "deb https://dl.yarnpkg.com/debian/ stable main" | tee /etc/apt/sources.list.d/yarn.list
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive && \
      apt-get -y install --no-install-recommends \
      musl-tools clang llvm nodejs yarn

RUN update-ca-certificates

ENV CC_aarch64_unknown_linux_musl=clang
ENV AR_aarch64_unknown_linux_musl=llvm-ar
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-Clink-self-contained=yes -Clinker=rust-lld"

ARG ARCH
RUN rustup target add ${ARCH}-unknown-linux-musl

WORKDIR /usr/src/
RUN USER=root cargo new app

WORKDIR /usr/src/app/
COPY Cargo.toml Cargo.lock /usr/src/app/

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# This is a dummy build to get the dependencies cached.
RUN cargo build --target ${ARCH}-unknown-linux-musl --release

# Now copy in the rest of the sources
COPY . /usr/src/app/

# This is the actual application build.
RUN cargo build --target ${ARCH}-unknown-linux-musl --release


FROM alpine AS runtime

WORKDIR /app/

ARG ARCH
COPY --from=builder /usr/src/app/conf/default.json /app/conf/default.json
COPY --from=builder /usr/src/app/migrations/ /app/migrations/
COPY --from=builder /usr/src/app/templates/ /app/templates/
COPY --from=builder /usr/src/app/static/ /app/static/
COPY --from=builder /usr/src/app/target/${ARCH}-unknown-linux-musl/release/exposed /usr/bin/exposed

ENTRYPOINT [ "/usr/bin/exposed" ]
