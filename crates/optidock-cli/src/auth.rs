use anyhow::{anyhow, bail, Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use oracle_rs::{Config, Connection};
use serde::{Deserialize, Serialize};
use std::env;

const ORACLE_USER_ENV: &str = "OPTIDOCK_ORACLE_USER";
const ORACLE_PASSWORD_ENV: &str = "OPTIDOCK_ORACLE_PASSWORD";
const ORACLE_CONNECT_STRING_ENV: &str = "OPTIDOCK_ORACLE_CONNECT_STRING";
const AUTH_TABLE_NAME: &str = "OPTIDOCK_OPERATORS";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthAccount {
    pub full_name: String,
    pub email: String,
    pub workspace: String,
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct OracleAuthConfig {
    pub user: String,
    pub password: String,
    pub connect_string: String,
}

#[derive(Debug, Clone)]
pub struct ParsedConnectString {
    pub host: String,
    pub port: u16,
    pub service_name: String,
}

#[derive(Debug, Clone)]
pub struct AuthDoctorReport {
    pub env_ready: bool,
    pub connection_ready: bool,
    pub detail: String,
}

pub fn load_oracle_auth_config() -> Result<OracleAuthConfig> {
    let user = read_required_env(ORACLE_USER_ENV)?;
    let password = read_required_env(ORACLE_PASSWORD_ENV)?;
    let connect_string = read_required_env(ORACLE_CONNECT_STRING_ENV)?;

    Ok(OracleAuthConfig {
        user,
        password,
        connect_string,
    })
}

pub async fn signup_operator(
    full_name: String,
    email: String,
    workspace: String,
    password: String,
) -> Result<AuthAccount> {
    let config = load_oracle_auth_config()?;
    let connection = connect(&config).await?;
    ensure_auth_table(&connection).await?;

    let password_hash = hash_password(&password)?;
    let insert_result = connection
        .execute(
            &format!(
                "INSERT INTO {AUTH_TABLE_NAME} (email, full_name, workspace, password_hash) VALUES (:1, :2, :3, :4)"
            ),
            &[
                email.as_str().into(),
                full_name.as_str().into(),
                workspace.as_str().into(),
                password_hash.as_str().into(),
            ],
        )
        .await;

    match insert_result {
        Ok(_) => {
            connection.commit().await?;
            Ok(AuthAccount {
                full_name,
                email,
                workspace,
                password_hash,
            })
        }
        Err(error) => {
            if is_duplicate_error(&error.to_string()) {
                bail!("an operator account already exists for that email");
            }
            Err(error).context("failed to create Oracle-backed operator account")
        }
    }
}

pub async fn login_operator(email: &str, password: &str) -> Result<AuthAccount> {
    let config = load_oracle_auth_config()?;
    let connection = connect(&config).await?;
    ensure_auth_table(&connection).await?;

    let result = connection
        .query(
            &format!(
                "SELECT full_name, email, workspace, password_hash FROM {AUTH_TABLE_NAME} WHERE email = :1"
            ),
            &[email.into()],
        )
        .await
        .context("failed to query Oracle-backed operator account")?;

    let Some(row) = result.rows.first() else {
        bail!("login failed");
    };

    let account = AuthAccount {
        full_name: row
            .get_string(0)
            .ok_or_else(|| anyhow!("missing full_name in Oracle auth row"))?,
        email: row
            .get_string(1)
            .ok_or_else(|| anyhow!("missing email in Oracle auth row"))?,
        workspace: row
            .get_string(2)
            .ok_or_else(|| anyhow!("missing workspace in Oracle auth row"))?,
        password_hash: row
            .get_string(3)
            .ok_or_else(|| anyhow!("missing password_hash in Oracle auth row"))?,
    };

    if !verify_password(password, &account.password_hash)? {
        bail!("login failed");
    }

    Ok(account)
}

pub async fn doctor_report() -> AuthDoctorReport {
    let config = match load_oracle_auth_config() {
        Ok(config) => config,
        Err(error) => {
            return AuthDoctorReport {
                env_ready: false,
                connection_ready: false,
                detail: error.to_string(),
            };
        }
    };

    let connection_result = async {
        let conn = connect(&config).await?;
        ensure_auth_table(&conn).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    match connection_result {
        Ok(()) => AuthDoctorReport {
            env_ready: true,
            connection_ready: true,
            detail: format!("connected to {}", config.connect_string),
        },
        Err(error) => AuthDoctorReport {
            env_ready: true,
            connection_ready: false,
            detail: error.to_string(),
        },
    }
}

async fn connect(config: &OracleAuthConfig) -> Result<Connection> {
    let parsed = parse_connect_string(&config.connect_string)?;
    let oracle_config = Config::new(
        parsed.host,
        parsed.port,
        parsed.service_name,
        config.user.clone(),
        config.password.clone(),
    );

    Connection::connect_with_config(oracle_config)
        .await
        .with_context(|| format!("failed to connect to Oracle at {}", config.connect_string))
}

async fn ensure_auth_table(connection: &Connection) -> Result<()> {
    let create_table = format!(
        "BEGIN
            EXECUTE IMMEDIATE '
                CREATE TABLE {AUTH_TABLE_NAME} (
                    email VARCHAR2(320) PRIMARY KEY,
                    full_name VARCHAR2(255) NOT NULL,
                    workspace VARCHAR2(255) NOT NULL,
                    password_hash VARCHAR2(512) NOT NULL,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
                )
            ';
        EXCEPTION
            WHEN OTHERS THEN
                IF SQLCODE != -955 THEN
                    RAISE;
                END IF;
        END;"
    );

    connection
        .execute(&create_table, &[])
        .await
        .context("failed to ensure Oracle auth table exists")?;
    connection.commit().await?;
    Ok(())
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
        .context("failed to hash operator password")
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool> {
    let parsed_hash =
        PasswordHash::new(password_hash).context("stored password hash is invalid")?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn is_duplicate_error(message: &str) -> bool {
    message.contains("ORA-00001") || message.to_ascii_lowercase().contains("unique constraint")
}

pub fn parse_connect_string(connect_string: &str) -> Result<ParsedConnectString> {
    let trimmed = connect_string.trim();
    let (host_port, service_name) = trimmed
        .split_once('/')
        .ok_or_else(|| anyhow!("Oracle connect string must look like host:port/service_name"))?;
    let (host, port) = host_port
        .split_once(':')
        .ok_or_else(|| anyhow!("Oracle connect string must look like host:port/service_name"))?;
    let port = port
        .parse::<u16>()
        .with_context(|| format!("invalid Oracle port in connect string: {port}"))?;

    if host.trim().is_empty() || service_name.trim().is_empty() {
        bail!("Oracle connect string must include host and service name");
    }

    Ok(ParsedConnectString {
        host: host.trim().to_string(),
        port,
        service_name: service_name.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{hash_password, parse_connect_string, verify_password};

    #[test]
    fn parses_basic_connect_string() {
        let parsed = parse_connect_string("db.freesql.com:1521/23ai_34ui2").unwrap();
        assert_eq!(parsed.host, "db.freesql.com");
        assert_eq!(parsed.port, 1521);
        assert_eq!(parsed.service_name, "23ai_34ui2");
    }

    #[test]
    fn rejects_invalid_connect_string() {
        assert!(parse_connect_string("db.freesql.com/23ai_34ui2").is_err());
        assert!(parse_connect_string("db.freesql.com:abc/23ai_34ui2").is_err());
    }

    #[test]
    fn hashes_and_verifies_passwords() {
        let password = "correct horse battery staple";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong password", &hash).unwrap());
    }
}
