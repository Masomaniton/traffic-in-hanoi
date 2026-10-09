use axum::http::{HeaderMap, HeaderValue, header::SET_COOKIE};
use uuid::Uuid;

pub type SessionId = String;

pub fn existing(headers: &HeaderMap) -> Option<SessionId> {
    headers
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            part.trim()
                .strip_prefix("traffic_session=")
                .map(ToOwned::to_owned)
        })
}

pub fn establish(headers: &HeaderMap) -> (SessionId, Option<HeaderValue>) {
    if let Some(existing) = existing(headers) {
        return (existing, None);
    }
    let value = Uuid::new_v4().to_string();
    let cookie = HeaderValue::from_str(&format!(
        "traffic_session={value}; Path=/; HttpOnly; SameSite=Lax"
    ))
    .expect("valid cookie");
    (value, Some(cookie))
}

pub fn cookie_header(cookie: Option<HeaderValue>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(cookie) = cookie {
        headers.insert(SET_COOKIE, cookie);
    }
    headers
}
