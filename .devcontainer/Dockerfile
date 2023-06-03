FROM mcr.microsoft.com/devcontainers/rust:0-1-bullseye

RUN curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | apt-key add -
RUN curl -fsSL https://deb.nodesource.com/setup_current.x | sudo -E bash -
RUN echo "deb https://dl.yarnpkg.com/debian/ stable main" | tee /etc/apt/sources.list.d/yarn.list
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
        postgresql-client nodejs yarn netcat
RUN update-alternatives --install /usr/bin/lldb-server-11.0.1 lldb-server /usr/lib/llvm-11/bin/lldb-server-11.0.1 100

CMD ["sleep", "infinity"]