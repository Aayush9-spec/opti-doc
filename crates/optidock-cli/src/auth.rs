use anyhow::{anyhow, bail, Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use optidock_core::{ChatContextRecord, NewChatContextRecord};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::env;

const SUPABASE_URL_ENV: &str = "NEXT_PUBLIC_SUPABASE_URL";
const SUPABASE_KEY_ENV: &str = "NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY";
const USERS_TABLE_NAME: &str = "optidock_users";
const CHAT_CONTEXT_TABLE_NAME: &str = "optidock_chat_context";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthAccount {
    pub user_id: Option<i64>,
    pub full_name: String,
    pub email: String,
    pub workspace: String,
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct SupabaseAuthConfig {
    pub url: String,
    pub publishable_key: String,
}

#[derive(Debug, Clone)]
pub struct AuthDoctorReport {
    pub env_ready: bool,
    pub connection_ready: bool,
    pub detail: String,
}

#[derive(Debug, Deserialize)]
struct SupabaseUserRow {
    user_id: i64,
    email: String,
    full_name: String,
    workspace_name: String,
    password_hash: String,
}

#[derive(Debug, Serialize)]
struct NewSupabaseUserRow<'a> {
    email: &'a str,
    full_name: &'a str,
    workspace_name: &'a str,
    password_hash: &'a str,
}

#[derive(Debug, Deserialize)]
struct SupabaseChatContextRow {
    context_id: i64,
    user_id: i64,
    session_key: Option<String>,
    context_label: Option<String>,
    context_payload: Option<String>,
    prompt_text: String,
    response_text: Option<String>,
    created_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct NewSupabaseChatContextRow<'a> {
    user_id: i64,
    session_key: Option<&'a str>,
    context_label: Option<&'a str>,
    context_payload: Option<&'a str>,
    prompt_text: &'a str,
    response_text: Option<&'a str>,
}

pub fn load_auth_config() -> Result<SupabaseAuthConfig> {
    let url = read_required_env(SUPABASE_URL_ENV)?;
    let publishable_key = read_required_env(SUPABASE_KEY_ENV)?;

    Ok(SupabaseAuthConfig {
        url: url.trim_end_matches('/').to_string(),
        publishable_key,
    })
}

pub async fn signup_operator(
    full_name: String,
    email: String,
    workspace: String,
    password: String,
) -> Result<AuthAccount> {
    let config = load_auth_config()?;
    let client = supabase_client();

    if find_user_by_email(&client, &config, &email).await?.is_some() {
        bail!("an operator account already exists for that email");
    }

    let password_hash = hash_password(&password)?;
    let row = NewSupabaseUserRow {
        email: &email,
        full_name: &full_name,
        workspace_name: &workspace,
        password_hash: &password_hash,
    };

    let rows: Vec<SupabaseUserRow> = client
        .post(rest_url(&config, USERS_TABLE_NAME))
        .headers(supabase_headers(&config, true)?)
        .json(&row)
        .send()
        .await
        .context("failed to create Supabase-backed operator account")?
        .error_for_status()
        .context("Supabase rejected operator signup")?
        .json()
        .await
        .context("failed to decode Supabase signup response")?;

    let inserted = rows
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Supabase did not return created user"))?;

    Ok(user_row_to_account(inserted))
}

pub async fn login_operator(email: &str, password: &str) -> Result<AuthAccount> {
    let config = load_auth_config()?;
    let client = supabase_client();
    let Some(row) = find_user_by_email(&client, &config, email).await? else {
        bail!("login failed");
    };

    if !verify_password(password, &row.password_hash)? {
        bail!("login failed");
    }

    Ok(user_row_to_account(row))
}

pub async fn store_chat_context(record: NewChatContextRecord) -> Result<ChatContextRecord> {
    let config = load_auth_config()?;
    let client = supabase_client();
    let user = find_user_by_email(&client, &config, &record.email)
        .await?
        .ok_or_else(|| anyhow!("no Supabase user found for email {}", record.email))?;

    let row = NewSupabaseChatContextRow {
        user_id: user.user_id,
        session_key: record.session_key.as_deref(),
        context_label: record.context_label.as_deref(),
        context_payload: record.context_payload.as_deref(),
        prompt_text: &record.prompt_text,
        response_text: record.response_text.as_deref(),
    };

    let rows: Vec<SupabaseChatContextRow> = client
        .post(rest_url(&config, CHAT_CONTEXT_TABLE_NAME))
        .headers(supabase_headers(&config, true)?)
        .json(&row)
        .send()
        .await
        .context("failed to store Supabase chat context")?
        .error_for_status()
        .context("Supabase rejected chat context insert")?
        .json()
        .await
        .context("failed to decode Supabase chat context response")?;

    let inserted = rows
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Supabase did not return stored chat context"))?;

    Ok(chat_row_to_record(inserted))
}

pub async fn recent_chat_context(email: &str, limit: usize) -> Result<Vec<ChatContextRecord>> {
    let config = load_auth_config()?;
    let client = supabase_client();
    let user = find_user_by_email(&client, &config, email)
        .await?
        .ok_or_else(|| anyhow!("no Supabase user found for email {email}"))?;

    let rows: Vec<SupabaseChatContextRow> = client
        .get(rest_url(&config, CHAT_CONTEXT_TABLE_NAME))
        .headers(supabase_headers(&config, false)?)
        .query(&[
            ("user_id", format!("eq.{}", user.user_id)),
            (
                "select",
                "context_id,user_id,session_key,context_label,context_payload,prompt_text,response_text,created_at"
                    .to_string(),
            ),
            ("order", "created_at.desc".to_string()),
            ("limit", limit.max(1).to_string()),
        ])
        .send()
        .await
        .context("failed to query Supabase chat context")?
        .error_for_status()
        .context("Supabase rejected chat context query")?
        .json()
        .await
        .context("failed to decode Supabase chat context rows")?;

    Ok(rows.into_iter().map(chat_row_to_record).collect())
}

pub async fn doctor_report() -> AuthDoctorReport {
    let config = match load_auth_config() {
        Ok(config) => config,
        Err(error) => {
            return AuthDoctorReport {
                env_ready: false,
                connection_ready: false,
                detail: error.to_string(),
            };
        }
    };

    let client = supabase_client();
    match client
        .get(rest_url(&config, USERS_TABLE_NAME))
        .headers(match supabase_headers(&config, false) {
            Ok(headers) => headers,
            Err(error) => {
                return AuthDoctorReport {
                    env_ready: true,
                    connection_ready: false,
                    detail: error.to_string(),
                };
            }
        })
        .query(&[("select", "user_id"), ("limit", "1")])
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => AuthDoctorReport {
            env_ready: true,
            connection_ready: true,
            detail: format!("connected to Supabase at {}", config.url),
        },
        Ok(response) if response.status() == StatusCode::NOT_FOUND => AuthDoctorReport {
            env_ready: true,
            connection_ready: false,
            detail: format!("Supabase table `{USERS_TABLE_NAME}` was not found"),
        },
        Ok(response) => AuthDoctorReport {
            env_ready: true,
            connection_ready: false,
            detail: format!("Supabase returned {}", response.status()),
        },
        Err(error) => AuthDoctorReport {
            env_ready: true,
            connection_ready: false,
            detail: error.to_string(),
        },
    }
}

async fn find_user_by_email(
    client: &Client,
    config: &SupabaseAuthConfig,
    email: &str,
) -> Result<Option<SupabaseUserRow>> {
    let rows: Vec<SupabaseUserRow> = client
        .get(rest_url(config, USERS_TABLE_NAME))
        .headers(supabase_headers(config, false)?)
        .query(&[
            ("email", format!("eq.{email}")),
            (
                "select",
                "user_id,email,full_name,workspace_name,password_hash".to_string(),
            ),
            ("limit", "1".to_string()),
        ])
        .send()
        .await
        .context("failed to query Supabase user")?
        .error_for_status()
        .context("Supabase rejected user lookup")?
        .json()
        .await
        .context("failed to decode Supabase user lookup")?;

    Ok(rows.into_iter().next())
}

fn supabase_client() -> Client {
    Client::new()
}

fn rest_url(config: &SupabaseAuthConfig, table_name: &str) -> String {
    format!("{}/rest/v1/{table_name}", config.url)
}

fn supabase_headers(
    config: &SupabaseAuthConfig,
    return_representation: bool,
) -> Result<reqwest::header::HeaderMap> {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

    let mut headers = HeaderMap::new();
    headers.insert("apikey", HeaderValue::from_str(&config.publishable_key)?);
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", config.publishable_key))?,
    );

    if return_representation {
        headers.insert("Prefer", HeaderValue::from_static("return=representation"));
    }

    Ok(headers)
}

fn user_row_to_account(row: SupabaseUserRow) -> AuthAccount {
    AuthAccount {
        user_id: Some(row.user_id),
        full_name: row.full_name,
        email: row.email,
        workspace: row.workspace_name,
        password_hash: row.password_hash,
    }
}

fn chat_row_to_record(row: SupabaseChatContextRow) -> ChatContextRecord {
    ChatContextRecord {
        context_id: Some(row.context_id),
        user_id: row.user_id,
        session_key: row.session_key,
        context_label: row.context_label,
        context_payload: row.context_payload,
        prompt_text: row.prompt_text,
        response_text: row.response_text,
        created_at: row.created_at,
    }
}

fn read_required_env(name: &str) -> Result<String> {
    let value = env::var(name).with_context(|| format!("{name} is not set"))?;
    let trimmed = value.trim();

    if trimmed.is_empty() {
        bail!("{name} is empty");
    }

    Ok(trimmed.to_string())
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| anyhow!("failed to hash operator password: {error}"))
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|error| anyhow!("stored password hash is invalid: {error}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password};

    #[test]
    fn hashes_and_verifies_passwords() {
        let password = "correct horse battery staple";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong password", &hash).unwrap());
    }
}
