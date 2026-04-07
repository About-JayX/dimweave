use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TelegramResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BotUser {
    pub id: i64,
    pub username: Option<String>,
    pub first_name: String,
}

#[derive(Debug, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub chat: TelegramChat,
    pub from: Option<TelegramUser>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TelegramChat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TelegramUser {
    pub id: i64,
    pub username: Option<String>,
    pub first_name: String,
}

fn api_url(token: &str, method: &str) -> String {
    format!("https://api.telegram.org/bot{token}/{method}")
}

pub async fn get_me(client: &Client, token: &str) -> anyhow::Result<BotUser> {
    let resp: TelegramResponse<BotUser> = client
        .get(api_url(token, "getMe"))
        .send()
        .await?
        .json()
        .await?;
    resp.result.ok_or_else(|| {
        anyhow::anyhow!("getMe failed: {}", resp.description.unwrap_or_default())
    })
}

pub async fn get_updates(
    client: &Client,
    token: &str,
    offset: Option<i64>,
    timeout_secs: u64,
) -> anyhow::Result<Vec<TelegramUpdate>> {
    let mut params = vec![("timeout", timeout_secs.to_string())];
    if let Some(off) = offset {
        params.push(("offset", off.to_string()));
    }
    let resp: TelegramResponse<Vec<TelegramUpdate>> = client
        .get(api_url(token, "getUpdates"))
        .query(&params)
        .timeout(std::time::Duration::from_secs(timeout_secs + 5))
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.result.unwrap_or_default())
}

pub async fn send_message(
    client: &Client,
    token: &str,
    chat_id: i64,
    text: &str,
) -> anyhow::Result<()> {
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
    });
    let resp: TelegramResponse<serde_json::Value> = client
        .post(api_url(token, "sendMessage"))
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    if !resp.ok {
        anyhow::bail!(
            "sendMessage failed: {}",
            resp.description.unwrap_or_default()
        );
    }
    Ok(())
}
