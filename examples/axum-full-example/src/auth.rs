// Author: 金书记
//
//! 认证相关代码

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use crate::AppState;

// ==================== 请求/响应类型 ====================

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_info: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub nickname: String,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteUserRequest {
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ManageUserRequest {
    pub user_id: String,
    pub action: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".to_string(),
            data: Some(data),
        }
    }
    
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            code: -1,
            message: message.into(),
            data: None,
        }
    }
}

// ==================== 错误处理 ====================

pub enum ApiError {
    Unauthorized(String),
    Forbidden(String),
    BadRequest(String),
    InternalError(String),
}

impl From<sa_token_plugin_axum::SaTokenError> for ApiError {
    fn from(err: sa_token_plugin_axum::SaTokenError) -> Self {
        match err {
            sa_token_plugin_axum::SaTokenError::NotLogin => {
                ApiError::Unauthorized("User not logged in".to_string())
            }
            sa_token_plugin_axum::SaTokenError::PermissionDenied
            | sa_token_plugin_axum::SaTokenError::PermissionDeniedDetail(_) => {
                ApiError::Forbidden("Permission denied".to_string())
            }
            sa_token_plugin_axum::SaTokenError::RoleDenied(_) => {
                ApiError::Forbidden("Role required".to_string())
            }
            _ => ApiError::InternalError(format!("Authentication error: {}", err)),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, 401, msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, 403, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, 400, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, 500, msg),
        };
        
        let body = Json(serde_json::json!({
            "code": code,
            "message": message,
            "data": serde_json::Value::Null,
        }));
        
        (status, body).into_response()
    }
}

// ==================== 登录接口 ====================

/// 从环境变量读取密码配置（仅用于演示，生产环境应使用数据库）
fn get_demo_password(username: &str) -> Option<String> {
    match username {
        "admin" => Some(std::env::var("DEMO_ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".to_string())),
        "user" => Some(std::env::var("DEMO_USER_PASSWORD").unwrap_or_else(|_| "user123".to_string())),
        "guest" => Some(std::env::var("DEMO_GUEST_PASSWORD").unwrap_or_else(|_| "guest123".to_string())),
        _ => None,
    }
}

/// 恒定时间比较（防时序攻击）
fn ct_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, ApiError> {
    // 验证用户名密码（实际应该查询数据库）
    let user_id = match req.username.as_str() {
        username @ ("admin" | "user" | "guest") => {
            // 从环境变量读取密码，使用恒定时间比较
            let expected = get_demo_password(username)
                .ok_or_else(|| ApiError::Unauthorized("用户名或密码错误".to_string()))?;
            if !ct_eq(&req.password, &expected) {
                return Err(ApiError::Unauthorized("用户名或密码错误".to_string()));
            }
            username
        }
        _ => {
            return Err(ApiError::Unauthorized("用户名或密码错误".to_string()));
        }
    };
    
    // 生成token
    let token = state.sa_token.manager
        .login(user_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("登录失败: {}", e)))?;
    
    // 获取用户权限和角色（使用 StpUtil）
    let permissions = sa_token_plugin_axum::StpUtil::get_permissions(user_id).await;
    let roles = sa_token_plugin_axum::StpUtil::get_roles(user_id).await;
    
    tracing::info!(
        "✅ 用户 {} 登录成功，权限: {:?}, 角色: {:?}", 
        user_id, permissions, roles
    );
    
    let user_info = UserInfo {
        id: user_id.to_string(),
        username: req.username.clone(),
        nickname: match user_id {
            "admin" => "管理员",
            "user" => "普通用户",
            "guest" => "访客",
            _ => "未知",
        }.to_string(),
        email: Some(format!("{}@example.com", req.username)),
    };
    
    let response = LoginResponse {
        token: token.to_string(),
        user_info,
    };
    
    Ok(Json(ApiResponse::success(response)))
}
