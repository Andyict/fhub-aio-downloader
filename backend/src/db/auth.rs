use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Admin => "admin",
            UserRole::User => "user",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "admin" => UserRole::Admin,
            _ => UserRole::User,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUser {
    pub id: String,
    pub username: String,
    pub role: UserRole,
    pub is_active: bool,
    pub must_change_password: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub id: String,
    pub user_id: String,
    pub expires_at: String,
}
