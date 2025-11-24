FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye
ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8

# Install additional packages
# protobuf-compiler: Required for tonic-build
# cmake: Required for prost-build/protobuf-src if needed
# lldb: For debugging
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    protobuf-compiler \
    cmake \
    lldb

# Update rust to ensure we have the latest stable version
RUN rustup update stable

# Create the vscode user
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME 
