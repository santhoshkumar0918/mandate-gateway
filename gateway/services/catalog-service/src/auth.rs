use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::AppState;

/// Authenticated identity carried in the request after the auth middleware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Account id (subject).
    pub sub: String,
    /// One of "merchant" | "agent" | "admin".
    pub role: String,
    /// Tenant id (merchant_id / agent_id); None for admin.
    pub tenant_id: Option<String>,
    /// Expiry (unix epoch seconds).
    pub exp: i64,
}

/// Issues a signed HS256 JWT. The secret must be the configured JWT_SECRET.
pub fn issue_token(secret: &[u8], claims: &Claims) -> Result<String, jsonwebtoken::errors::Error> {
    let key = EncodingKey::from_secret(secret);
    encode(&Header::default(), claims, &key)
}

/// Verifies a signed HS256 JWT, returning the claims or an error.
pub fn verify_token(secret: &[u8], token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let key = DecodingKey::from_secret(secret);
    let data = decode::<Claims>(token, &key, &Validation::default())?;
    Ok(data.claims)
}

/// Authenticated request principal, extracted from the Bearer token.
///
/// Any handler that takes `AuthUser` as an argument becomes protected: a
/// missing or invalid token yields 401. The downstream handler reads the
/// claims without re-verifying.
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    #[allow(clippy::manual_async_fn)]
    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let header = parts
                .headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .ok_or((StatusCode::UNAUTHORIZED, "missing authorization header"))?;

            let token = header
                .strip_prefix("Bearer ")
                .ok_or((StatusCode::UNAUTHORIZED, "authorization must be a Bearer token"))?;

            match verify_token(&state.jwt_secret, token) {
                Ok(claims) => Ok(AuthUser(claims)),
                Err(_) => Err((StatusCode::UNAUTHORIZED, "invalid or expired token")),
            }
        }
    }
}
