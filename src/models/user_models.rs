use serde_json::Value;
use serde::{Serialize, Deserialize};

// Need to move this to a separate file because it is not exclusive to user routes
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseModel {
    pub status: u16,
    pub message: String,
    pub data: Value
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterUserModel {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginUserModel {
    pub username: String,
    pub password: String,
}