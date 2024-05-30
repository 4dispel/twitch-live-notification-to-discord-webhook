use axum::{extract::{ State}, handler::Handler, http::{header::HeaderMap, header, StatusCode}, response::IntoResponse, routing::post, BoxError, Router, debug_handler};
use reqwest::Client;
use serde_json::json;
mod twitch_messages;
mod message_verification;
use shuttle_runtime::SecretStore;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct AppState {
    twitch_client_id: String,
    twitch_access_token: String,
    twitch_client_secret: String,
    twitch_event_secret: String,
    request_client: Client,
    discord_webhook_url: String,
}

#[debug_handler]
async fn handle_callback(
    State(app): State<AppState>,
    headers: HeaderMap,
    body: String,
) ->  axum::response::Response {
    if message_verification::verify_message(&headers, &body, &app.twitch_event_secret).is_err() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let message_type = match headers.get("twitch-eventsub-message-type") {
        Some(x) if x.to_str().unwrap().contains("webhook_callback_verification") => twitch_messages::TwitchEventsubMessage::WebhookCallbackVerification,
        Some(x) if x.to_str().unwrap().contains("notification") => twitch_messages::TwitchEventsubMessage::Notification,
        Some(x) if x.to_str().unwrap().contains("revocation") => twitch_messages::TwitchEventsubMessage::Revocation,
        _ => return StatusCode::BAD_REQUEST.into_response(),

    };
    match message_type {
        twitch_messages::TwitchEventsubMessage::WebhookCallbackVerification => handle_verification_callback(&app, headers, body),
        twitch_messages::TwitchEventsubMessage::Notification => handle_notification_callback(&app, headers, body).await,
        twitch_messages::TwitchEventsubMessage::Revocation => handle_revocation_callback(&app, headers, body).await,
        _ => StatusCode::OK.into_response(),
    }
}

fn handle_verification_callback(app: &AppState, headers: HeaderMap, body: String) -> axum::response::Response {
    let challenge_request = match serde_json::from_str::<twitch_messages::ChallengeRequest>(body.as_str()) {
        Ok(x) => x,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let challenge = challenge_request.challenge;
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/plain")], challenge).into_response()
}

async fn handle_notification_callback(app: &AppState, headers: HeaderMap, body: String) -> axum::response::Response {
    println!("headers: {:#?}\n body: {}", headers, body);
    let user_name = match serde_json::from_str::<twitch_messages::NotificationMessage>(body.as_str()) {
        Ok(x) => x.event.broadcaster_user_login,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if let Err(err) = send_discord_to_webhook(format!("{} went live\nhttps://twitch.tv/{}", user_name, user_name), &app.discord_webhook_url.as_str(), &app.request_client).await {
        println!("coudln't send message to discord err: {err}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    StatusCode::OK.into_response()
}

async fn handle_revocation_callback(app: &AppState, headers: HeaderMap, body: String) -> axum::response::Response {
    println!("headers: {:#?}\n body: {}", headers, body);
    let rev_message = match serde_json::from_str::<twitch_messages::RevocationMessage>(body.as_str()) {
        Ok(x) => x,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let user_id = rev_message.condition.broadcaster_user_id;
    let reason = rev_message.status;
    if let Err(err) = send_discord_to_webhook(format!("revoced subscription to user id: {}  reason: {}", user_id, reason), &app.discord_webhook_url.as_str(), &app.request_client).await {
        println!("coudln't send message to discord err: {err}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    StatusCode::OK.into_response()
}

async fn send_discord_to_webhook(message: String, url: &str, client: &Client) -> Result<(), BoxError>{
    let mut map = HashMap::new();
    map.insert(
        String::from("content"),
        message
    );
    client.post(url).json(&map).send().await?;
    Ok(())
}

async fn get_twitch_access_token(client_id : &str, client_secret: &str, client: &reqwest::Client) -> Result<String, BoxError> {
    Ok(String::from(client
        .post(format!("https://id.twitch.tv/oauth2/token?client_id={}&client_secret={}&grant_type=client_credentials",
            client_id,
            client_secret))
        .send().await?
        .json::<twitch_messages::AppTokenMessage>().await?
        .access_token))
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> shuttle_axum::ShuttleAxum {
    let twitch_client_id = secrets.get("TWITCH_CLIENT_ID").unwrap();
    let twitch_client_secret = secrets.get("TWITCH_CLIENT_SECRET").unwrap();
    let twitch_event_secret = secrets.get("TWITCH_EVENT_SECRET").unwrap();
    let discord_webhook_url = secrets.get("DISCORD_WEBHOOK_URL").unwrap();
    let request_client = Client::new();
    let twitch_access_token = get_twitch_access_token(&twitch_client_id, &twitch_client_secret, &request_client).await.unwrap();
    let app = AppState {
        twitch_client_id,
        twitch_access_token,
        twitch_client_secret,
        twitch_event_secret,
        request_client,
        discord_webhook_url,
    };

    let router = Router::new().route("/callback", post(handle_callback)).with_state(app);

    Ok(router.into())
}
