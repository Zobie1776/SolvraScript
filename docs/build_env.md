# SolvraOS Build Environment

This document outlines the necessary dependencies, environment variables, and setup instructions to build and test the SolvraOS ecosystem.

## Dependencies

- **Rust:** The project is built using Rust. It is recommended to use `rustup` to install and manage Rust versions. The specific version is defined in `rust-toolchain.toml`.
- **Node.js:** The IDE and other parts of the ecosystem use Node.js. It is recommended to use `nvm` to manage Node.js versions. The specific version is defined in `.nvmrc`.
- **pnpm:** The project uses `pnpm` as the package manager for Node.js.
- **Docker:** Docker is required to build and run the project in a containerized environment.

## Environment Variables

- `CARGO_NET_OFFLINE=true`: This variable is used to ensure that the project can be built and tested offline. It is recommended to set this variable in your shell profile.

## Setup

1.  **Clone the repository:**
    ```bash
    git clone --recurse-submodules https://github.com/Solvra/SolvraOS.git
    cd SolvraOS
    ```

2.  **Install Rust:**
    `rustup` will automatically install the correct toolchain version based on the `rust-toolchain.toml` file.

3.  **Install Node.js and pnpm:**
    ```bash
    nvm install
    npm install -g pnpm
    ```

4.  **Install dependencies:**
    ```bash
    pnpm install
    ```

5.  **Build the project:**
    ```bash
    cargo build --workspace
    ```

6.  **Run tests:**
    ```bash
    cargo test --workspace
    ```

## Docker

A Dockerfile is provided to create a deterministic build environment.

```bash
docker build -t solvra_build .
docker run -v $(pwd):/build solvra_build cargo test
```
