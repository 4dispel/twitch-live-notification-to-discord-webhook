use serde::{Deserialize, Serialize};

// https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/#client-credentials-grant-flow
// just the access_token
#[derive(Debug, Deserialize)]
pub struct AppTokenMessage {
    pub access_token: String,
}

//https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/#client-credentials-grant-flow
#[derive(Debug)]
pub enum TwitchEventsubMessage {
    Notification,
    WebhookCallbackVerification,
    Revocation,
}

//https://dev.twitch.tv/docs/eventsub/handling-webhook-events/#responding-to-a-challenge-request
#[derive(Debug, Deserialize)]
pub struct ChallengeRequest {
    pub challenge: String,
}

#[derive(Debug, Deserialize)]
pub struct NotificationMessage {
    pub event: Event,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    pub broadcaster_user_login: String,
}

#[derive(Debug, Deserialize)]
pub struct RevocationMessage {
    pub condition: Condition,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct Condition {
    pub broadcaster_user_id: String,
}
