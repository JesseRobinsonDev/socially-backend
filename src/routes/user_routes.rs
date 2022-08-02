use reqwest::header::USER_AGENT;
use rocket_contrib::json::Json;
use serde_json::json;
use serde_json::Value;
use redis::Commands;
use uuid::Uuid;
use sha2::{Sha256, Digest};
use hex;
use std::collections::HashMap;
use std::env;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

pub use crate::controllers::user_controllers;
pub use crate::models::user_models;
pub use crate::config::connection::get_redis_connection;

/*
    let new_user = user_models::RegisterUserRequestModel {
        username: user.username.clone(),
        password: user.password.clone()
    };
*/

#[post("/register", format = "json", data = "<user>")]
pub fn register_user(user: Json<user_models::RegisterUserModel>) -> Json<user_models::ResponseModel> {

    let mut con = match get_redis_connection() {
        Ok(con) => con,
        Err(_) => return Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        }),
    };

    let iter: redis::Iter<String> = match con.hscan_match("usernames", user.username.clone()) {
        Ok(iter) => iter,
        Err(_) => return Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        }),
    };

    let length = iter.count();

    if length > 0 {
        return Json(user_models::ResponseModel {
            status: 400,
            message: format!("Username already exists"),
            data: json!({"error": true})
        })
    }

    let mut hasher = Sha256::new();
    hasher.update(user.password.clone());
    let hash = hasher.finalize();
    let encoded_pass = hex::encode(hash);

    let id = Uuid::new_v4();

    let _ : () = con.hset(id.to_string(), "id", id.to_string()).unwrap();
    let _ : () = con.hset(id.to_string(), "username", user.username.clone()).unwrap();
    let _ : () = con.hset(id.to_string(), "password", encoded_pass).unwrap();

    let _ : () = con.hset("usernames", user.username.clone(), id.to_string()).unwrap();

    Json(user_models::ResponseModel {
        status: 200,
        message: format!("Successfully registered user"),
        data: json!({"UserID": id.to_string()})
    })

}

#[post("/login", format = "json", data = "<user>")]
pub fn user_login(user: Json<user_models::LoginUserModel>) -> Json<user_models::ResponseModel> {

    let mut con = match get_redis_connection() {
        Ok(con) => con,
        Err(_) => return Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        }),
    };

    let id: String = match con.hget("usernames", user.username.clone()) {
        Ok(id) => id,
        Err(_) => return Json(user_models::ResponseModel {
            status: 404,
            message: format!("User does not exist"),
            data: json!({"error": true})
        }),
    };

    let mut hasher = Sha256::new();
    hasher.update(user.password.clone());
    let hash = hasher.finalize();
    let encoded_pass = hex::encode(hash);

    let password: String = match con.hget(&id, "password") {
        Ok(password) => password,
        Err(_) => return Json(user_models::ResponseModel {
            status: 500,
            message: format!("User password does not exist"),
            data: json!({"error": true})
        }),
    };

    if password != encoded_pass {
        return Json(user_models::ResponseModel {
            status: 401,
            message: format!("Incorrect password"),
            data: json!({"error": true})
        });
    }

    Json(user_models::ResponseModel {
        status: 200,
        message: format!("Successfully logged in"),
        data: json!({"UserID": id})
    })

}

#[get("/connect/<id>/spotify/url?<redirect_uri>")]
pub fn get_user_spotify_url(id: String, redirect_uri: String) -> Json<user_models::ResponseModel> {

    let mut con = match get_redis_connection() {
        Ok(con) => con,
        Err(_) => return Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        }),
    };

    let _ = match user_controllers::check_db_for_user(&mut con, id.clone()) {
        Ok(_) => true,
        Err(e) => return e
    };

    let rand_string = user_controllers::generate_random_string(64);

    let _ : () = con.hset(id.clone(), "spotify_state", &rand_string).unwrap();

    Json(user_models::ResponseModel {
        status: 200,
        message: format!("Returned connection URL"),
        data: json!({"URL": format!("https://accounts.spotify.com/authorize?response_type=code&state={}&client_id={}&redirect_uri={}", rand_string, env::var("SPOTIFY_CLIENT_ID").unwrap(), redirect_uri)})
    })

}

#[post("/connect/<id>/spotify?<code>&<state>")]
pub fn connect_user_to_spotify(id: String, code: String, state: String) -> Json<user_models::ResponseModel> {

    let client = reqwest::blocking::Client::new();

    let spotify_tokens = match user_controllers::get_spotify_tokens(code.clone()) {
        Ok(tokens) => tokens,
        Err(e) => return e 
    };

    let access_token = match user_controllers::get_json_str("access_token", spotify_tokens.clone()) {
        Ok(token) => token,
        Err(e) => return e,
    };

    let refresh_token = match user_controllers::get_json_str("refresh_token", spotify_tokens.clone()) {
        Ok(token) => token,
        Err(e) => return e,
    };

    let mut con = match get_redis_connection() {
        Ok(con) => con,
        Err(_) => return Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        }),
    };

    let _ = match user_controllers::check_db_for_user(&mut con, id.clone()) {
        Ok(_) => true,
        Err(e) => return e,
    };

    let spotify_state: String = con.hget(id.clone(), "spotify_state").unwrap();

    if state != spotify_state {
        return Json(user_models::ResponseModel {
            status: 401,
            message: format!("Invalid state"),
            data: json!({"error": true})
        })
    }

    let _ : () = con.hdel(id.clone(), "spotify_state").unwrap();
    match user_controllers::set_redis_hash_string(&mut con, id.clone(), "spotify_access_token", &access_token) {
        Ok(_) => (),
        Err(e) => return e,
    };
    match user_controllers::set_redis_hash_string(&mut con, id.clone(), "spotify_refresh_token", &refresh_token) {
        Ok(_) => (),
        Err(e) => return e,
    };

    let spotify_user = match user_controllers::get_spotify_user_data(access_token.clone()) {
        Ok(tokens) => tokens,
        Err(e) => return e 
    };

    let user_id = match user_controllers::get_json_str("id", spotify_user.clone()) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let username = match user_controllers::get_json_str("display_name", spotify_user.clone()) {
        Ok(name) => name,
        Err(e) => return e,
    };

    match user_controllers::set_redis_hash_string(&mut con, id.clone(), "spotify_id", &user_id) {
        Ok(_) => (),
        Err(e) => return e,
    };
    match user_controllers::set_redis_hash_string(&mut con, id.clone(), "spotify_name", &username) {
        Ok(_) => (),
        Err(e) => return e,
    };

    Json(user_models::ResponseModel {
        status: 200,
        message: format!("Successfully connected account to spotify"),
        data: json!({"success": true})
    })

}

#[get("/connect/<id>/reddit/url?<redirect_uri>")]
pub fn get_user_reddit_url(id: String, redirect_uri: String) -> Json<user_models::ResponseModel> {

    let mut con = match get_redis_connection() {
        Ok(con) => con,
        Err(_) => return Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        }),
    };

    let _ = match user_controllers::check_db_for_user(&mut con, id.clone()) {
        Ok(_) => true,
        Err(e) => return e
    };

    let rand_string = user_controllers::generate_random_string(64);

    let _ : () = con.hset(id.clone(), "reddit_state", &rand_string).unwrap();

    Json(user_models::ResponseModel {
        status: 200,
        message: format!("Returned connection URL"),
        data: json!({"URL":
        format!("https://www.reddit.com/api/v1/authorize?client_id={}&response_type=code&state={}&redirect_uri={}&duration=permanent&scope=identity,read", env::var("REDDIT_CLIENT_ID").unwrap(), &rand_string, redirect_uri)})
    })

}

#[post("/connect/<id>/reddit?<code>&<state>")]
pub fn connect_user_to_reddit(id: String, code: String, state: String) -> Json<user_models::ResponseModel> {

    let client = reqwest::blocking::Client::new();

    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("code", &code.as_str());
    params.insert("redirect_uri", "http://localhost:3000/me/connect/reddit");

    let res = client.post("https://www.reddit.com/api/v1/access_token")
    .form(&params).header("Authorization", "")
    .send().unwrap();

    let reddit_tokens: Value = serde_json::from_str(&res.text().unwrap()).unwrap();
    let access_token = &reddit_tokens.get("access_token").unwrap().as_str().unwrap();

    let mut con = match get_redis_connection() {
        Ok(con) => con,
        Err(_) => return Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        }),
    };

    let iter: redis::Iter<String> = match con.scan_match(id.clone()) {
        Ok(iter) => iter,
        Err(_) => return Json(user_models::ResponseModel {
            status: 500,
            message: format!("Server Error"),
            data: json!({"error": true})
        }),
    };

    let length = iter.count();

    if length == 0 {
        return Json(user_models::ResponseModel {
            status: 400,
            message: format!("User does not exist"),
            data: json!({"error": true})
        })
    }

    let reddit_state: String = con.hget(id.clone(), "reddit_state").unwrap();

    if state != reddit_state {
        return Json(user_models::ResponseModel {
            status: 401,
            message: format!("Invalid state"),
            data: json!({"error": true})
        })
    }

    let _ : () = con.hdel(id.clone(), "reddit_state").unwrap();
    let _ : () = con.hset(id.clone(), "reddit_access_token", &reddit_tokens.get("access_token").unwrap().as_str().unwrap()).unwrap();
    let _ : () = con.hset(id.clone(), "reddit_refresh_token", &reddit_tokens.get("refresh_token").unwrap().as_str().unwrap()).unwrap();

    let res2 = client.get("https://oauth.reddit.com/api/v1/me")
    .header("Authorization", format!("Bearer {}", &access_token)).header(USER_AGENT, "socially/1.0")
    .send().unwrap();

    let reddit_user: Value = serde_json::from_str(&res2.text().unwrap()).unwrap();

    let _ : () = con.hset(id.clone(), "reddit_id", &reddit_user.get("id").unwrap().as_str().unwrap()).unwrap();
    let _ : () = con.hset(id.clone(), "reddit_display_name", &reddit_user.get("name").unwrap().as_str().unwrap()).unwrap();

    Json(user_models::ResponseModel {
        status: 200,
        message: format!("Successfully connected account to reddit"),
        data: json!({"success": true})
    })

}

#[get("/connect/<id>/twitter?<redirect_uri>")]
pub fn get_user_twitter_url(id: String, redirect_uri: String) -> Json<user_models::ResponseModel> {

    Json(user_models::ResponseModel {
        status: 200,
        message: format!("Returned connection URL"),
        data: json!({"URL":
        format!("https://twitter.com/i/oauth2/authorize?
        response_type=code
        &client_id={}
        &redirect_uri={}
        &scope=tweet.read%20users.read%20follows.read%20follows.write
        &state=SSTTRRIINNGG
        &code_challenge=MeTyRrUJxsfsiqnfdXWWxmj63ZfpnFUNfHKz2SPROek&code_challenge_method=s256", env::var("TWITTER_CLIENT_ID").unwrap(), redirect_uri)})
    })

}

#[post("/connect/<id>/twitter?<code>")]
pub fn connect_user_to_twitter(id: String, code: String) -> String {

    let client = reqwest::blocking::Client::new();

    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("code", &code.as_str());
    params.insert("redirect_uri", "http://localhost:3000/me/connect/twitter");
    params.insert("code_verifier", "09kP8eh45jzIYtpnwkowCYzQO9DX4_zm8TiIqYM-5D0");

    let res = client.post("https://api.twitter.com/2/oauth2/token")
    .form(&params).header("Authorization", "")
    .send().unwrap();
    
    format!("integer: {:?}", &res.text())

}

#[get("/get/<id>")]
pub fn get_user_by_id(id: String) -> String {
    format!("id: {}", id)
}

#[delete("/delete/<id>")]
pub fn delete_user_by_id(id: String) -> String {
    format!("id: {}", id)
}
