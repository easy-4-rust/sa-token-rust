// Author: 金书记
//
//! StpUtil 无参数方法演示

use sa_token_plugin_axum::StpUtil;
use axum::{response::Json, http::StatusCode};
use serde_json::json;

/// 演示：获取当前 token
pub async fn demo_get_token() -> Result<Json<serde_json::Value>, StatusCode> {
    match StpUtil::get_token_value() {
        Ok(token) => Ok(Json(json!({
            "code": 200,
            "data": { "token": token.as_str() }
        }))),
        Err(_) => Err(StatusCode::UNAUTHORIZED)
    }
}

/// 演示：检查是否登录
pub async fn demo_is_login() -> Json<serde_json::Value> {
    let is_logged_in = StpUtil::is_login_current();
    Json(json!({
        "code": 200,
        "data": { "is_logged_in": is_logged_in }
    }))
}

/// 演示：获取 login_id（String）
pub async fn demo_get_login_id_string() -> Result<Json<serde_json::Value>, StatusCode> {
    match StpUtil::get_login_id_as_string().await {
        Ok(login_id) => Ok(Json(json!({
            "code": 200,
            "data": { "login_id": login_id }
        }))),
        Err(_) => Err(StatusCode::UNAUTHORIZED)
    }
}

/// 演示：获取 login_id（i64）
pub async fn demo_get_login_id_long() -> Result<Json<serde_json::Value>, StatusCode> {
    match StpUtil::get_login_id_as_long().await {
        Ok(user_id) => Ok(Json(json!({
            "code": 200,
            "data": { "user_id": user_id }
        }))),
        Err(_) => Err(StatusCode::UNAUTHORIZED)
    }
}

/// 演示：登出当前会话
pub async fn demo_logout() -> Result<Json<serde_json::Value>, StatusCode> {
    match StpUtil::logout_current().await {
        Ok(_) => Ok(Json(json!({
            "code": 200,
            "message": "登出成功"
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
