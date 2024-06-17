use axum::{
    debug_handler,
    extract::{Extension, FromRef, Multipart, State},
    http::{
        header::{self, HeaderMap},
        Response, StatusCode,
    },
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    BoxError, Router,
};
use reqwest::Client;
mod message_verification;
mod sub_management;
mod twitch_messages;
use axum_extra::{
    extract::cookie::{Cookie, Key, PrivateCookieJar, SameSite},
};
use shuttle_runtime::SecretStore;
use std::collections::HashMap;
use std::sync::Arc;
use tera::{Context, Tera};

type Template = Arc<Tera>;

#[derive(Debug, Clone)]
struct AppState {
    twitch_client_id: String,
    twitch_access_token: String,
    twitch_client_secret: String,
    twitch_event_secret: String,
    server_url: String,
    request_client: Client,
    discord_webhook_url: String,
    password: String,
    key: Key,
}

impl FromRef<AppState> for Key {
    fn from_ref(input: &AppState) -> Self {
        input.key.clone()
    }
}

#[debug_handler]
async fn handle_callback(
    State(app): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> axum::response::Response {
    if message_verification::verify_message(&headers, &body, &app.twitch_event_secret).is_err() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let message_type = match headers.get("twitch-eventsub-message-type") {
        Some(x)
            if x.to_str()
                .unwrap()
                .contains("webhook_callback_verification") =>
        {
            twitch_messages::TwitchEventsubMessage::WebhookCallbackVerification
        }
        Some(x) if x.to_str().unwrap().contains("notification") => {
            twitch_messages::TwitchEventsubMessage::Notification
        }
        Some(x) if x.to_str().unwrap().contains("revocation") => {
            twitch_messages::TwitchEventsubMessage::Revocation
        }
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    match message_type {
        twitch_messages::TwitchEventsubMessage::WebhookCallbackVerification => {
            handle_verification_callback(body)
        }
        twitch_messages::TwitchEventsubMessage::Notification => {
            handle_notification_callback(&app, headers, body).await
        }
        twitch_messages::TwitchEventsubMessage::Revocation => {
            handle_revocation_callback(&app, headers, body).await
        }
    }
}

fn handle_verification_callback(
    body: String,
) -> axum::response::Response {
    let challenge_request =
        match serde_json::from_str::<twitch_messages::ChallengeRequest>(body.as_str()) {
            Ok(x) => x,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
    let challenge = challenge_request.challenge;
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain")],
        challenge,
    )
        .into_response()
}

async fn handle_notification_callback(
    app: &AppState,
    headers: HeaderMap,
    body: String,
) -> axum::response::Response {
    println!("headers: {:#?}\n body: {}", headers, body);
    let user_name =
        match serde_json::from_str::<twitch_messages::NotificationMessage>(body.as_str()) {
            Ok(x) => x.event.broadcaster_user_login,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
    if let Err(err) = send_discord_to_webhook(
        format!("{} went live\nhttps://twitch.tv/{}", user_name, user_name),
        &app.discord_webhook_url.as_str(),
        &app.request_client,
    )
    .await
    {
        println!("coudln't send message to discord err: {err}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    StatusCode::OK.into_response()
}

async fn handle_revocation_callback(
    app: &AppState,
    headers: HeaderMap,
    body: String,
) -> axum::response::Response {
    println!("headers: {:#?}\n body: {}", headers, body);
    let rev_message =
        match serde_json::from_str::<twitch_messages::RevocationMessage>(body.as_str()) {
            Ok(x) => x,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
    let user_id = rev_message.condition.broadcaster_user_id;
    let reason = rev_message.status;
    if let Err(err) = send_discord_to_webhook(
        format!(
            "revoced subscription to user id: {}  reason: {}",
            user_id, reason
        ),
        &app.discord_webhook_url.as_str(),
        &app.request_client,
    )
    .await
    {
        println!("coudln't send message to discord err: {err}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    StatusCode::OK.into_response()
}

async fn send_discord_to_webhook(
    message: String,
    url: &str,
    client: &Client,
) -> Result<(), BoxError> {
    let mut map = HashMap::new();
    map.insert(String::from("content"), message);
    client
        .post(url)
        .json(&map)
        .send().await?;
    Ok(())
}

async fn get_twitch_access_token(
    client_id: &str,
    client_secret: &str,
    client: &reqwest::Client,
) -> Result<String, BoxError> {
    Ok(String::from(client
        .post(format!("https://id.twitch.tv/oauth2/token?client_id={}&client_secret={}&grant_type=client_credentials",
            client_id,
            client_secret))
        .send().await?
        .json::<twitch_messages::AppTokenMessage>().await?
        .access_token))
}

fn correct_password(jar: &PrivateCookieJar, password: &String) -> bool {
    if let Some(cookie) = jar.get("password") {
        if cookie.value_trimmed() != password.as_str() {
            false
        } else {
            true
        }
    } else {
        false
    }
}

async fn index(
    State(app): State<AppState>,
    Extension(templates): Extension<Template>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    if !correct_password(&jar, &app.password) {
        return Redirect::to("/login").into_response();
    }
    let mut ctx = Context::new();
    let user_cards = sub_management::get_cards(
        &app.request_client,
        &app.twitch_client_id,
        &app.twitch_access_token,
    )
    .await;
    ctx.insert("users", &user_cards);
    Html(templates.render("index", &ctx).unwrap()).into_response()
}

async fn reverify(
    State(mut app): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    if !correct_password(&jar, &app.password) {
        return Redirect::to("/login").into_response();
    }
    app.twitch_access_token = get_twitch_access_token(&app.twitch_client_id, &app.twitch_client_secret, &app.request_client).await.unwrap();
    Redirect::to("/").into_response()    

}

async fn styles() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(include_str!("../public/styles.css").to_owned())
        .unwrap()
}

async fn get_login(Extension(templates): Extension<Template>) -> impl IntoResponse {
    Html(templates.render("login", &Context::new()).unwrap())
}

async fn post_login(jar: PrivateCookieJar, multipart: Multipart) -> impl IntoResponse {
    let password = from_multipart(multipart, "password").await.unwrap();
    let cookie = Cookie::build(("password", password))
        .secure(true)
        .same_site(SameSite::Strict)
        .http_only(true)
        .build();
    (jar.add(cookie), Redirect::to("/"))
}

async fn post_remove(
    jar: PrivateCookieJar,
    State(app): State<AppState>,
    multipart: Multipart,
) -> impl IntoResponse {
    if !correct_password(&jar, &app.password) {
        return Redirect::to("/login").into_response();
    }
    if let Some(id) = from_multipart(multipart, "eventsub_id").await {
        sub_management::remove_sub(
            id,
            &app.request_client,
            &app.twitch_client_id,
            &app.twitch_access_token,
            ).await;
    }
    Redirect::to("/").into_response()    
}

async fn post_add(
    jar: PrivateCookieJar,
    State(app): State<AppState>,
    multipart: Multipart,
) -> impl IntoResponse {
    if !correct_password(&jar, &app.password) {
        return Redirect::to("/login").into_response();
    }
    if let Some(login) = from_multipart(multipart, "login").await {
        sub_management::readd_sub(
            login,
            &app.request_client,
            &app.twitch_client_id,
            &app.twitch_access_token,
            &app.twitch_event_secret,
            &app.server_url,
            ).await;
    } 
    Redirect::to("/").into_response()    
}


async fn from_multipart(mut multipart: Multipart, key: &str) -> Option<String> {
    while let Some(field) = multipart.next_field().await.ok()? {
        if key == field.name()?.to_string() {
            return Some(field.text().await.ok()?);
        }
    }
    None
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_axum::ShuttleAxum {
    let twitch_client_id = secrets.get("TWITCH_CLIENT_ID").unwrap();
    let twitch_client_secret = secrets.get("TWITCH_CLIENT_SECRET").unwrap();
    let twitch_event_secret = secrets.get("TWITCH_EVENT_SECRET").unwrap();
    let server_url = secrets.get("SERVER_URL").unwrap();
    let discord_webhook_url = secrets.get("DISCORD_WEBHOOK_URL").unwrap();
    let password = secrets.get("PASSWORD").unwrap();
    let request_client = Client::new();
    let twitch_access_token =
        get_twitch_access_token(&twitch_client_id, &twitch_client_secret, &request_client)
            .await
            .unwrap();
    let app = AppState {
        twitch_client_id,
        twitch_access_token,
        twitch_client_secret,
        twitch_event_secret,
        request_client,
        server_url,
        discord_webhook_url,
        password,
        key: Key::generate(),
    };
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("base.html", include_str!("../templates/base.html")),
        ("index", include_str!("../templates/index.html")),
        ("login", include_str!("../templates/login.html")),
        ("macros.html", include_str!("../templates/macros.html")),
    ])
    .unwrap();

    let router = Router::new()
        .route("/callback", post(handle_callback))
        .route("/", get(index))
        .route("/login", get(get_login).post(post_login))
        .route("/remove", post(post_remove))
        .route("/add", post(post_add))
        .route("/styles.css", get(styles))
        .route("/reverify", post(reverify))
        .layer(Extension(Arc::new(tera)))
        .with_state(app);

    Ok(router.into())
}
