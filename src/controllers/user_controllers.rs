use std::collections::HashMap;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use redis::Commands;
use rocket_contrib::json::Json;
use serde::de::value;
use serde_json::json;
use serde_json::Value;
use reqwest::blocking::Client;

pub use crate::models::user_models;

pub fn generate_random_string(length: usize) -> String {

    thread_rng()
    .sample_iter(&Alphanumeric)
    .take(length)
    .map(char::from)
    .collect()

}

pub fn check_db_for_user(redis_con: &mut redis::Connection, id: String) -> Result<bool, Json<user_models::ResponseModel>> {

    let iter: redis::Iter<String> = match redis_con.scan_match(id.clone()) {
        Ok(iter) => iter,
        Err(_) => return Err(Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        })),
    };

    let length = iter.count();

    if length == 0 {
        return Err(Json(user_models::ResponseModel {
            status: 400,
            message: format!("User does not exist"),
            data: json!({"error": true})
        }))
    }

    return Ok(true)

}

pub fn get_json_str(field: &str, value: Value) -> Result<String, Json<user_models::ResponseModel>> {

    let val = value.clone();

    let unwrapped_field = match val.get(field) {
        Some(val) => val,
        None => return Err(Json(user_models::ResponseModel {
            status: 404,
            message: format!("Field {} not found", field),
            data: json!({"error": true})
        })),
    };

    let stringed_field = match unwrapped_field.as_str() {
        Some(str) => str,
        None => return Err(Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        })),
    };

    return Ok(stringed_field.clone().to_string())

}

pub fn set_redis_hash_string(redis_con: &mut redis::Connection, id: String, field: &str, value: &str) -> Result<bool, Json<user_models::ResponseModel>> {

    let _: () = match redis_con.hset(id.clone(), field, value) {
        Ok(t) => t,
        Err(_) => return Err(Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        })),
    };

    return Ok(true)

}

pub fn get_redis_hash_string(redis_con: &mut redis::Connection, id: String, field: &str) -> Result<String, Json<user_models::ResponseModel>> {
    return Ok("hi".to_string())
}

pub fn delete_redis_hash_value(redis_con: &mut redis::Connection, id: String, field: &str) -> Result<bool, Json<user_models::ResponseModel>> {
    return Ok(true)
}

pub fn get_spotify_tokens(code: String) -> Result<Value, Json<user_models::ResponseModel>> {

    let client = Client::new();

    let mut params = HashMap::new();

    params.insert("grant_type", "authorization_code");
    params.insert("code", &code.as_str());
    params.insert("redirect_uri", "http://localhost:3000/me/connect/spotify");

    let res = match client.post("https://accounts.spotify.com/api/token")
    .form(&params).header("Authorization", "")
    .send() {
        Ok(res) => res,
        Err(_) => return Err(Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        })),
    };

    Ok(serde_json::from_str(&res.text().unwrap()).unwrap())

}

pub fn get_spotify_user_data(token: String) -> Result<Value, Json<user_models::ResponseModel>> {
    
    let client = Client::new();

    let res = match client.get("https://api.spotify.com/v1/me")
    .header("Authorization", format!("Bearer {}", &token))
    .send() {
        Ok(res) => res,
        Err(_) => return Err(Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        })),
    };

    Ok(serde_json::from_str(&res.text().unwrap()).unwrap())

}
