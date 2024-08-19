FROM gingersociety/rust-rocket-api-builder:latest as builder

# Create a new directory for the app
WORKDIR /app

# Copy the current directory contents into the container at /app
COPY . .

# Set executable permissions for the build script
RUN chmod +x /app/build.sh

ARG GINGER_TOKEN
ENV GINGER_TOKEN=$GINGER_TOKEN

RUN ginger-connector connect stage-k8

# Execute the build script
RUN /app/build.sh

# Second stage: Create the minimal runtime image
FROM gingersociety/rust-rocket-api-runner:latest

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/MetadataService /app/

# Set the working directory
WORKDIR /app


# Run the executable when the container starts
ENTRYPOINT ["./MetadataService"]
