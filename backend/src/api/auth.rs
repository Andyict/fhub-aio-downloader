use std::sync::Arc;

use axum::{
    extract::{Path, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::{AppUser, UserRole},
    AppState,
};

pub const SESSION_COOKIE: &str = "fhub_session";
const SESSION_HOURS: i64 = 24 * 7;

pub fn public_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/setup-status", get(setup_status))
        .route("/setup-admin", post(setup_admin))
}

pub fn protected_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/me", get(me))
        .route("/change-password", post(change_password))
}

pub fn admin_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route("/users/:id", patch(update_user).delete(delete_user))
}

#[derive(Clone)]
pub struct AuthUser(pub AppUser);

#[derive(Serialize)]
struct MeResponse {
    authenticated: bool,
    user: AppUser,
}

#[derive(Serialize)]
struct UsersResponse {
    users: Vec<AppUser>,
}

#[derive(Serialize)]
struct ActionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

#[derive(Serialize)]
struct SetupStatusResponse {
    setup_required: bool,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct CreateUserRequest {
    username: String,
    password: String,
    role: Option<String>,
}

#[derive(Deserialize)]
struct UpdateUserRequest {
    password: Option<String>,
    role: Option<String>,
    is_active: Option<bool>,
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
struct SetupAdminRequest {
    username: String,
    password: String,
    confirm_password: String,
}

fn parse_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for item in cookie_header.split(';') {
        let mut parts = item.trim().splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        if key == name {
            return Some(value.to_string());
        }
    }
    None
}

fn new_session_token() -> String {
    let mut bytes = [0u8; 32];
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    bytes[..16].copy_from_slice(a.as_bytes());
    bytes[16..].copy_from_slice(b.as_bytes());
    URL_SAFE_NO_PAD.encode(bytes)
}

fn session_cookie_value(token: &str) -> String {
    format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        SESSION_COOKIE,
        token,
        SESSION_HOURS * 3600,
        if std::env::var("FHUB_AUTH_SECURE_COOKIE").unwrap_or_default() == "1" {
            "; Secure"
        } else {
            ""
        }
    )
}

fn clear_session_cookie_value() -> String {
    format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        SESSION_COOKIE
    )
}

pub fn seed_admin_if_needed(_state: &Arc<AppState>) {
    // Disabled on purpose: first-run admin is created interactively from the FHUB UI.
}

fn is_first_run(state: &Arc<AppState>) -> Result<bool, StatusCode> {
    let users = state
        .db
        .list_app_users()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // The database always seeds a disabled internal "shared library" pseudo-user.
    // First-run setup should only be considered complete after a real active admin exists.
    Ok(!users
        .iter()
        .any(|user| user.role == UserRole::Admin && user.is_active))
}

pub fn current_user_from_headers(state: &Arc<AppState>, headers: &HeaderMap) -> Result<Option<AppUser>, StatusCode> {
    let Some(token) = parse_cookie(headers, SESSION_COOKIE) else {
        return Ok(None);
    };
    let user = state
        .db
        .get_session_user_by_token(&token)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter(|u| u.is_active);
    if user.is_some() {
        let new_expiry = (Utc::now() + Duration::hours(SESSION_HOURS)).to_rfc3339();
        let _ = state.db.touch_session(&token, &new_expiry);
    }
    Ok(user)
}

pub async fn auth_required(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user = current_user_from_headers(&state, request.headers())?.ok_or(StatusCode::UNAUTHORIZED)?;
    request.extensions_mut().insert(AuthUser(user));
    Ok(next.run(request).await)
}

pub async fn admin_required(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user = current_user_from_headers(&state, request.headers())?.ok_or(StatusCode::UNAUTHORIZED)?;
    if user.role != UserRole::Admin {
        return Err(StatusCode::FORBIDDEN);
    }
    request.extensions_mut().insert(AuthUser(user));
    Ok(next.run(request).await)
}

async fn setup_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SetupStatusResponse>, StatusCode> {
    Ok(Json(SetupStatusResponse {
        setup_required: is_first_run(&state)?,
    }))
}

async fn setup_admin(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetupAdminRequest>,
) -> Result<Json<ActionResponse>, StatusCode> {
    if !is_first_run(&state)? {
        return Err(StatusCode::CONFLICT);
    }

    let username = payload.username.trim();
    let password = payload.password.trim();
    let confirm_password = payload.confirm_password.trim();

    if username.is_empty() || password.len() < 6 || confirm_password.len() < 6 {
        return Err(StatusCode::BAD_REQUEST);
    }

    if password != confirm_password {
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .db
        .create_app_user_with_password_policy(username, password, UserRole::Admin, false)
        .map_err(|_| StatusCode::CONFLICT)?;

    Ok(Json(ActionResponse {
        success: true,
        message: Some("FHUB admin workspace created".to_string()),
        code: None,
    }))
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, StatusCode> {
    let username = payload.username.trim();
    let password = payload.password.trim();
    if username.is_empty() || password.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (user, password_hash) = state
        .db
        .get_app_user_by_username(username)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !user.is_active || password_hash != crate::db::Db::hash_password(password) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = new_session_token();
    let expires_at = (Utc::now() + Duration::hours(SESSION_HOURS)).to_rfc3339();
    state
        .db
        .create_app_session(&user.id, &token, &expires_at)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .db
        .set_app_user_last_login(&user.id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie_value(&token))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    let fresh_user = state
        .db
        .get_app_user_by_id(&user.id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((headers, Json(MeResponse { authenticated: true, user: fresh_user })).into_response())
}

async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    if let Some(token) = parse_cookie(&headers, SESSION_COOKIE) {
        let _ = state.db.delete_session_by_token(&token);
    }
    let mut out = HeaderMap::new();
    out.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&clear_session_cookie_value())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok((out, Json(ActionResponse { success: true, message: Some("FHUB session closed".to_string()), code: None })).into_response())
}

async fn me(request: Request) -> Result<Json<MeResponse>, StatusCode> {
    let user = request
        .extensions()
        .get::<AuthUser>()
        .map(|u| u.0.clone())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(Json(MeResponse {
        authenticated: true,
        user,
    }))
}

async fn list_users(State(state): State<Arc<AppState>>) -> Result<Json<UsersResponse>, StatusCode> {
    let users = state
        .db
        .list_app_users()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(UsersResponse { users }))
}

async fn change_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<ActionResponse>, StatusCode> {
    let user = current_user_from_headers(&state, &headers)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let current_password = payload.current_password.trim();
    let new_password = payload.new_password.trim();

    if current_password.is_empty() || new_password.len() < 6 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (_fresh_user, password_hash) = state
        .db
        .get_app_user_by_username(&user.username)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if password_hash != crate::db::Db::hash_password(current_password) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    state
        .db
        .update_app_user_password(&user.id, new_password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ActionResponse {
        success: true,
        message: Some("FHUB account password updated".to_string()),
        code: None,
    }))
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<ActionResponse>, StatusCode> {
    let username = payload.username.trim();
    let password = payload.password.trim();
    if username.is_empty() || password.len() < 6 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let role = UserRole::from_str(payload.role.as_deref().unwrap_or("user"));
    state
        .db
        .create_app_user(username, password, role)
        .map_err(|_| StatusCode::CONFLICT)?;
    Ok(Json(ActionResponse {
        success: true,
        message: Some("FHUB workspace member created".to_string()),
        code: None,
    }))
}

async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<ActionResponse>, StatusCode> {
    if let Some(password) = payload.password.as_deref() {
        if password.trim().len() < 6 {
            return Err(StatusCode::BAD_REQUEST);
        }
        state
            .db
            .update_app_user_password(&id, password.trim())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        state
            .db
            .delete_sessions_by_user(&id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    if let Some(is_active) = payload.is_active {
        state
            .db
            .set_app_user_active(&id, is_active)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !is_active {
            state
                .db
                .delete_sessions_by_user(&id)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    if let Some(role) = payload.role.as_deref() {
        state
            .db
            .set_app_user_role(&id, UserRole::from_str(role))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(Json(ActionResponse {
        success: true,
        message: Some("FHUB workspace member updated".to_string()),
        code: None,
    }))
}

async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ActionResponse>, StatusCode> {
    state
        .db
        .delete_app_user(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ActionResponse {
        success: true,
        message: Some("FHUB workspace member removed".to_string()),
        code: None,
    }))
}
