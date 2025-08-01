use crate::history::{History, HistoryEntry};
use crate::identity::{AccessResponse, IdentityStore, Outcome};
use async_trait::async_trait;
use rocket::State;
use rocket::config::Config;
use rocket::http::Status;
use rocket::serde::json::Json;
use serde_derive::Deserialize;
use std::error::Error;
use std::time::SystemTime;
use tokio::sync::Mutex;

#[async_trait]
pub trait Callback: Send + Sync {
    async fn call(&self) -> Result<(), Box<dyn Error>>;
}

#[derive(Deserialize)]
pub struct ServerSettings {
    pub mount_point: String,
    pub port: u16,
}

pub struct Context {
    pub identity_store: Box<dyn IdentityStore>,
    pub history: Box<dyn History>,
}

pub async fn run(
    settings: ServerSettings,
    context: Context,
    callback: Box<dyn Callback>,
) -> Result<(), Box<dyn Error>> {
    let _ = rocket::custom(Config::figment().merge(("port", settings.port)))
        .mount(&settings.mount_point, routes![history, access])
        .manage(Mutex::new(context))
        .manage(callback)
        .launch()
        .await?;
    Ok(())
}

#[get("/access?<time_min>&<time_max>&<token>&<name>&<outcome>&<only_latest>")]
pub async fn history(
    time_min: Option<u64>,
    time_max: Option<u64>,
    token: Option<String>,
    name: Option<String>,
    outcome: Option<Outcome>,
    only_latest: Option<bool>,
    state: &State<Mutex<Context>>,
) -> Result<Json<Vec<HistoryEntry>>, (Status, String)> {
    let context = state.inner().lock().await;
    match context.history.query(
        time_min,
        time_max,
        token.as_ref().map(|x| &**x), // This map turns Option<&String> into Option<&str>
        name.as_ref().map(|x| &**x),
        outcome,
        only_latest.unwrap_or(false),
    ) {
        Err(e) => Err((Status::ServiceUnavailable, e.to_string())),
        Ok(result) => Ok(Json(result)),
    }
}

#[post("/access?<token>")]
pub async fn access(
    token: String,
    state: &State<Mutex<Context>>,
    callback: &State<Box<dyn Callback>>,
) -> Result<(Status, Json<AccessResponse>), (Status, String)> {
    let context = &mut state.inner().lock().await;
    let response = context
        .identity_store
        .access(&token)
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| (Status::InternalServerError, e.to_string()))?
        .as_secs();
    context
        .history
        .insert(HistoryEntry {
            time,
            token,
            response: response.clone(),
        })
        .map_err(|e| (Status::ServiceUnavailable, e.to_string()))?;
    let _ = context;
    let status = if response.outcome == Outcome::Success {
        callback
            .inner()
            .call()
            .await
            .map_err(|e| (Status::BadGateway, e.to_string()))?;
        Status::Ok
    } else {
        Status::Forbidden
    };
    Ok((status, Json(response)))
}
