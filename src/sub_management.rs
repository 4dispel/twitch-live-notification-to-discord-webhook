use reqwest::Client;
use std::collections::HashMap;
use serde::{Serialize, Deserialize, };
#[derive(Debug, Serialize)]
pub struct UserCard {
    pfp_link: String,
    user_login: String,
    eventsub_id: String,
    status: String
}

#[derive(Debug, Deserialize)]
struct EventSubs {
    data: Vec<EventSub>
}

#[derive(Debug, Deserialize, Clone)]
struct EventSub {
    id: String,
    status: String,
    condition: Condition,
}

#[derive(Debug, Deserialize, Clone)]
struct Condition {
    broadcaster_user_id: String,
}


#[derive(Debug, Deserialize)]
struct Users {
    data: Vec<User>,
}

#[derive(Debug, Deserialize, Clone)]
struct User {
    id: String,
    login: String,
    profile_image_url: String,
}

#[derive(Debug)]
enum UserIdentifier {
    ID(String),
    LOGIN(String),
}



pub async fn get_cards(
    client: &Client,
    twitch_client_id: &String,
    twitch_access_token: &String,
) -> Vec<UserCard> {
    let mut cards: Vec<UserCard> = Vec::new();
    for sub in get_subs(client, twitch_client_id, twitch_access_token).await.iter() {
        if let Some(card) = card_from_sub(sub.clone(), client, twitch_client_id, twitch_access_token).await {
            cards.push(card);
        }
    }
    cards
}

pub async fn remove_sub(
    sub_id: String, 
    client: &Client,
    twitch_client_id: &String,
    twitch_access_token: &String,
) {
    if client.delete(format!("https://api.twitch.tv/helix/eventsub/subscriptions?id={}", sub_id))
        .header("Client-Id", twitch_client_id)
        .header("Authorization", format!("Bearer {}", twitch_access_token))
        .send().await
        .is_err() {
            println!("couldn't remove sub");
    }
}

pub async fn readd_sub(
    login: String,
    client: &Client,
    twitch_client_id: &String,
    twitch_access_token: &String,
    twitch_eventsub_secret: &String,
    server_url: &String,
) {
    let subs = get_subs(client, twitch_client_id, twitch_access_token).await;
    if let Some(user) = get_user(UserIdentifier::LOGIN(login), client, twitch_client_id, twitch_access_token).await {
        for sub in subs {
            if sub.condition.broadcaster_user_id == user.id {
                remove_sub(sub.id, client, twitch_client_id, twitch_access_token).await;
            }
        }
        add_sub(user.id, client, twitch_client_id, twitch_access_token, twitch_eventsub_secret, server_url).await;
    }

}

async fn add_sub(
    id: String,
    client: &Client,
    twitch_client_id: &String,
    twitch_access_token: &String,
    twitch_eventsub_secret: &String,
    server_url: &String,
) {
    let callback_url = format!("{}/callback", server_url);
    let json_payload = serde_json::json!({
        "type": "stream.online",
        "version": "1",
        "condition" : {
            "broadcaster_user_id": id
        },
        "transport": {
            "method": "webhook",
            "callback": callback_url,
            "secret": twitch_eventsub_secret,
        }
    });
    if client.post("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Client-Id", twitch_client_id)
        .header("Authorization", format!("Bearer {}", twitch_access_token))
        .json(&json_payload)
        .send().await
        .is_err() {
            println!("couldn't create sub")
    }

}

async fn card_from_sub(
    sub: EventSub,
    client: &Client,
    twitch_client_id: &String,
    twitch_access_token: &String,
) -> Option<UserCard> {
    if let Some(user) = get_user(UserIdentifier::ID(sub.condition.broadcaster_user_id.clone()), client, twitch_client_id, twitch_access_token).await {
        Some(UserCard{
            pfp_link: user.profile_image_url,
            user_login: user.login,
            eventsub_id: sub.id,
            status: sub.status,
        })
    } else {
        None
    }
}

async fn get_subs(
    client: &Client,
    twitch_client_id: &String,
    twitch_access_token: &String,
) -> Vec<EventSub> {
    let response = client.get("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Client-Id", twitch_client_id)
        .header("Authorization", format!("Bearer {}", twitch_access_token))
        .send().await;
    if response.is_err() {
        return vec![];
    } 
    let eventsubs = response.unwrap().json::<EventSubs>().await;
    if eventsubs.is_err() {
        return vec![];
    }
    eventsubs.unwrap().data
}

async fn get_user(
    identifier: UserIdentifier, 
    client: &Client,
    twitch_client_id: &String,
    twitch_access_token: &String,
) -> Option<User> {
    let url = format!("https://api.twitch.tv/helix/users?{}", match identifier {
        UserIdentifier::ID(id) => format!("id={}", id),
        UserIdentifier::LOGIN(login) => format!("login={}", login),
    });
    let response = client.get(url)
        .header("Client-Id", twitch_client_id)
        .header("Authorization", format!("Bearer {}", twitch_access_token))
        .send().await;
    if response.is_err() {
        return None;
    }
    let users = response.unwrap().json::<Users>().await;
    if users.is_err() {
        return None;
    }
    Some(users.unwrap().data.get(0)?.clone())
}
