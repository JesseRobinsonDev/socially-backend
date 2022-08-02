use std::env;

pub fn get_redis_connection() -> Result<redis::Connection, redis::RedisError> {
    let client = match redis::Client::open(env::var("REDIS_URI").unwrap()) {
        Ok(client) => client,
        Err(_error) => panic!("Error getting client")
    };
    client.get_connection()
}