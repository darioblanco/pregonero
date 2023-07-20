# Use the official Rust image as build environment
FROM rust:latest AS build

# Create a new directory to hold the application code
WORKDIR /usr/src/app

# Copy the application code into the container
COPY . .

# Build the application
RUN cargo build --release

# Create a new image using a minimal Alpine runtime environment
FROM alpine:latest

# Install the CA certificates needed to make HTTPS requests
RUN apk add --no-cache ca-certificates

# Copy the application binary into the container
COPY --from=build /usr/src/app/target/release/sabi /usr/local/bin/sabi

# Set the environment variables needed by the application
ENV API_ADDRESS=0.0.0.0:3030
ENV LOG_LEVEL=info

# Expose the port used by the application
EXPOSE 3030

# Start the application when the container starts
CMD ["/usr/local/bin/sabi"]
