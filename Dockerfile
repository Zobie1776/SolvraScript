FROM rust:1.76

# Install Node.js and pnpm
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
RUN apt-get install -y nodejs
RUN npm install -g pnpm

# Create a non-root user
RUN useradd -ms /bin/bash user
USER user
WORKDIR /home/user/project

# Copy project files
COPY . .

# Install dependencies
RUN pnpm install

# Set offline mode for Cargo
ENV CARGO_NET_OFFLINE=true

# Run tests
CMD ["cargo", "test", "--workspace"]
