#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate rocket_cors;
extern crate serde;
extern crate redis;
extern crate dotenv;

use dotenv::dotenv;
use rocket::http::Method;
use rocket_cors::{AllowedHeaders, AllowedOrigins, Error, Cors, CorsOptions};

pub use crate::routes::user_routes;
pub mod config { pub mod connection; }
pub mod controllers { pub mod user_controllers; }
pub mod routes { pub mod user_routes; }
pub mod models { pub mod user_models; }


fn make_cors() -> Cors {
    let allowed_origins = AllowedOrigins::some_exact(&[
        "http://localhost:3000",
    ]);
    CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Post, Method::Put, Method::Patch, Method::Delete, Method::Options].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::All,
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .expect("error while building CORS")
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/user", routes![
            user_routes::register_user,
            user_routes::user_login,
            user_routes::get_user_spotify_url,
            user_routes::connect_user_to_spotify,
            user_routes::get_user_reddit_url,
            user_routes::connect_user_to_reddit,
            user_routes::get_user_twitter_url,
            user_routes::connect_user_to_twitter,
            user_routes::get_user_by_id,
            user_routes::delete_user_by_id,]).attach(make_cors())
}

fn main() -> Result<(), Error> {
    dotenv().ok();

    rocket().launch();

    Ok(())
}