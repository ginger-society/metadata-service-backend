use r2d2_redis::{r2d2::Pool, RedisConnectionManager};
use std::ops::Deref;

// Define a type alias for your Redis connection pool
type RedisPool = Pool<RedisConnectionManager>;

// Function to create and return a Redis connection pool
pub fn create_redis_pool(redis_url: &str) -> RedisPool {
    // Create a Redis connection manager
    let manager = RedisConnectionManager::new(redis_url).unwrap();

    // Create a pool with the manager
    Pool::builder()
        .max_size(15) // Set the maximum number of connections in the pool
        .build(manager)
        .unwrap()
}

// Rocket State to manage the Redis pool
#[derive(Debug)]
pub struct RedisPoolState(pub RedisPool);

// Implement Deref for RedisPoolState to access the pool easily
impl Deref for RedisPoolState {
    type Target = RedisPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
