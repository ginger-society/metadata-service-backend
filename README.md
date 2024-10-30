## Metadata service backend

DB : https://github.com/ginger-society/ginger-db

## 🔧 Building and Testing

### debug mode
> cargo run

### release mode
> cargo build --release && cargo run --release


### unit testing
> cargo test



Build : 

```sh
docker build . -t gingersociety/metadata-service-api-stage --platform=linux/amd64
```


Push : 

```sh
docker push gingersociety/metadata-service-api-stage
```



to restart the deployment / upgrade the pod image version 
```sh 

 kubectl rollout restart deployment metadata-service-api-deployment
```
TODO: move these to IAC repo
For building base builder image
```sh
docker build . -t gingersociety/rust-rocket-api-builder -f Dockerfile.builder --platform=linux/amd64

docker push gingersociety/rust-rocket-api-builder
```

For building base runner image
```sh
docker build . -t gingersociety/rust-rocket-api-runner -f Dockerfile.runner --platform=linux/amd64

docker push gingersociety/rust-rocket-api-runner
```