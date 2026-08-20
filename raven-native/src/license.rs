use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const API_BASE_URL: &str = "https://ravennotch.me/api";
const OFFLINE_GRACE_DAYS: i64 = 7;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LicenseCache {
    pub device_id: String,
    pub license_key: Option<String>,
    #[serde(default)]
    pub account_token: Option<String>,
    #[serde(default)]
    pub account_email: Option<String>,
    #[serde(default)]
    pub account_name: Option<String>,
    #[serde(default)]
    pub account_username: Option<String>,
    #[serde(default)]
    pub account_picture: Option<String>,
    pub status: String,
    pub trial_expires_at: Option<String>,
    pub last_checked_at: Option<String>,
    #[serde(default)]
    pub force_trial_expired_preview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub status: String,
    pub device_id: String,
    pub license_key: Option<String>,
    pub account_email: Option<String>,
    pub account_name: Option<String>,
    pub account_username: Option<String>,
    pub account_picture: Option<String>,
    pub trial_expires_at: Option<String>,
    pub message: Option<String>,
    pub force_trial_expired_preview: bool,
}

#[derive(Debug, Deserialize)]
struct ApiEntitlement {
    status: String,
    #[serde(rename = "licenseKey")]
    license_key: Option<String>,
    #[serde(rename = "deviceId")]
    device_id: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: Option<String>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiAccountStatus {
    status: String,
    purchased: Option<bool>,
    plan: Option<String>,
    message: Option<String>,
    error: Option<String>,
    user: Option<ApiAccountUser>,
}

#[derive(Debug, Deserialize)]
struct ApiAccountUser {
    email: Option<String>,
    name: Option<String>,
    username: Option<String>,
    picture: Option<String>,
}

fn remember_account_user(cache: &mut LicenseCache, user: Option<ApiAccountUser>) {
    if let Some(user) = user {
        if user.email.as_deref().is_some_and(|value| !value.trim().is_empty()) {
            cache.account_email = user.email;
        }
        if user.name.as_deref().is_some_and(|value| !value.trim().is_empty()) {
            cache.account_name = user.name;
        }
        if user.username.as_deref().is_some_and(|value| !value.trim().is_empty()) {
            cache.account_username = user.username;
        }
        if user.picture.as_deref().is_some_and(|value| !value.trim().is_empty()) {
            cache.account_picture = user.picture;
        }
    }
}

pub fn is_premium_locked(status: &LicenseStatus) -> bool {
    status.force_trial_expired_preview || status.status == "trial_expired"
}

pub fn status_label(status: &LicenseStatus) -> String {
    if status.force_trial_expired_preview {
        "Trial expired preview is on".to_string()
    } else if status.status == "paid_active" {
        if status.account_email.is_some() {
            "Raven Notch account is activated".to_string()
        } else {
            "Raven Notch is activated".to_string()
        }
    } else if status.status == "trial_active" {
        "Trial active".to_string()
    } else if status.status == "trial_expired" {
        "Trial period over".to_string()
    } else {
        "Checking license".to_string()
    }
}

pub fn set_force_trial_expired_preview(enabled: bool) -> Result<LicenseStatus, String> {
    let mut cache = load_cache()?;
    cache.force_trial_expired_preview = enabled;
    save_cache(&cache)?;
    Ok(cached_status(&cache, None))
}

pub fn get_license_status() -> Result<LicenseStatus, String> {
    let mut cache = load_cache()?;
    if cache.force_trial_expired_preview {
        cache.force_trial_expired_preview = false;
        save_cache(&cache)?;
    }

    if let Some(account_token) = cache.account_token.clone() {
        match post_account_status(&account_token, &cache.device_id) {
            Ok(account) if account.status == "active" && account.purchased.unwrap_or(false) => {
                cache.status = "paid_active".to_string();
                remember_account_user(&mut cache, account.user);
                cache.last_checked_at = Some(Utc::now().to_rfc3339());
                save_cache(&cache)?;
                return Ok(cached_status(&cache, Some("Lifetime purchase linked to this account.".to_string())));
            }
            Ok(account) if account.status == "device_mismatch" => {
                cache.status = "trial_expired".to_string();
                save_cache(&cache)?;
                return Ok(cached_status(&cache, account.message.or(account.error)));
            }
            Ok(account) => {
                remember_account_user(&mut cache, account.user);
                if !account.purchased.unwrap_or(false) && cache.license_key.is_none() {
                    cache.status = if has_trial_time(&cache) {
                        "trial_active".to_string()
                    } else {
                        "trial_expired".to_string()
                    };
                }
                save_cache(&cache)?;
            }
            Err(_error) if has_paid_grace(&cache) => {
                return Ok(cached_status(
                    &cache,
                    None,
                ));
            }
            Err(_) => {}
        }
    }
 
    if let Some(license_key) = cache.license_key.clone() {
        match post_entitlement(
            "/license/status",
            serde_json::json!({
                "licenseKey": license_key,
                "deviceId": cache.device_id,
            }),
        ) {
            Ok(entitlement) if entitlement.status == "paid_active" => {
                cache.status = "paid_active".to_string();
                cache.last_checked_at = Some(Utc::now().to_rfc3339());
                cache.license_key = entitlement.license_key.or(Some(license_key));
                save_cache(&cache)?;
                return Ok(cached_status(&cache, None));
            }
            Ok(entitlement) => {
                cache.status = entitlement.status;
                save_cache(&cache)?;
                return Ok(cached_status(
                    &cache,
                    entitlement.message.or(entitlement.error),
                ));
            }
            Err(_error) if has_paid_grace(&cache) => {
                return Ok(cached_status(
                    &cache,
                    None,
                ));
            }
            Err(error) => return Ok(cached_status(&cache, Some(error))),
        }
    }
 
    match post_entitlement(
        "/trial/start",
        serde_json::json!({
            "deviceId": cache.device_id,
        }),
    ) {
        Ok(entitlement) => {
            cache.status = entitlement.status;
            cache.trial_expires_at = entitlement.expires_at;
            cache.last_checked_at = Some(Utc::now().to_rfc3339());
            save_cache(&cache)?;
            Ok(cached_status(
                &cache,
                entitlement.message.or(entitlement.error),
            ))
        }
        Err(_error) if has_trial_time(&cache) => Ok(cached_status(
            &cache,
            None,
        )),
        Err(error) => {
            cache.status = if has_trial_time(&cache) {
                "trial_active".to_string()
            } else {
                "trial_expired".to_string()
            };
            save_cache(&cache)?;
            Ok(cached_status(&cache, Some(error)))
        }
    }
}

pub fn activate_license(license_key: String) -> Result<LicenseStatus, String> {
    let mut cache = load_cache()?;
    let clean_key = license_key.trim().to_uppercase();
    if clean_key.is_empty() {
        return Err("Enter a license key".to_string());
    }

    let entitlement = post_entitlement(
        "/license/activate",
        serde_json::json!({
            "licenseKey": clean_key,
            "deviceId": cache.device_id,
        }),
    )?;

    if entitlement.status != "paid_active" {
        let message = entitlement.message.or(entitlement.error).or_else(|| {
            Some(match entitlement.status.as_str() {
                "invalid" => "Invalid license key. Please check the key and try again.",
                "device_mismatch" => "This license key is already active on another device.",
                _ => "License activation failed. Please try again.",
            }
            .to_string())
        });
        return Ok(LicenseStatus {
            status: entitlement.status,
            device_id: entitlement.device_id.unwrap_or(cache.device_id),
            license_key: Some(clean_key),
            account_email: cache.account_email,
            account_name: cache.account_name,
            account_username: cache.account_username,
            account_picture: cache.account_picture,
            trial_expires_at: cache.trial_expires_at,
            message,
            force_trial_expired_preview: cache.force_trial_expired_preview,
        });
    }

    cache.status = "paid_active".to_string();
    cache.license_key = Some(clean_key);
    cache.last_checked_at = Some(Utc::now().to_rfc3339());
    save_cache(&cache)?;
    Ok(cached_status(&cache, None))
}

pub fn account_sign_in_url() -> String {
    format!("{API_BASE_URL}/auth/google/start?source=app")
}

pub fn sign_out_account() -> Result<LicenseStatus, String> {
    let mut cache = load_cache()?;
    cache.account_token = None;
    cache.account_email = None;
    cache.account_name = None;
    cache.account_username = None;
    cache.account_picture = None;
    cache.license_key = None; // Clean license key as well
    cache.status = if has_trial_time(&cache) {
        "trial_active".to_string()
    } else {
        "trial_expired".to_string()
    };
    save_cache(&cache)?;
    Ok(cached_status(&cache, Some("Signed out of Raven account.".to_string())))
}

pub fn connect_account_token(token: String) -> Result<LicenseStatus, String> {
    let mut cache = load_cache()?;
    let clean_token = token.trim().to_string();
    if clean_token.is_empty() {
        return Err("Missing account sign-in token".to_string());
    }

    let account = post_account_status(&clean_token, &cache.device_id)?;
    cache.account_token = Some(clean_token);
    remember_account_user(&mut cache, account.user);
    cache.last_checked_at = Some(Utc::now().to_rfc3339());

    if account.status == "active" && account.purchased.unwrap_or(false) {
        cache.status = "paid_active".to_string();
        save_cache(&cache)?;
        Ok(cached_status(&cache, Some("Lifetime purchase linked to this account.".to_string())))
    } else {
        if !account.purchased.unwrap_or(false) && cache.license_key.is_none() {
            cache.status = if has_trial_time(&cache) {
                "trial_active".to_string()
            } else {
                "trial_expired".to_string()
            };
        }
        save_cache(&cache)?;
        Ok(cached_status(
            &cache,
            account.message.or(account.error).or_else(|| {
                Some(match account.status.as_str() {
                    "no_purchase" => "Signed in, but this account has not purchased Raven Notch yet.",
                    "device_mismatch" => "This account is already active on another device.",
                    _ => "Signed in, but account activation is not complete.",
                }
                .to_string())
            }),
        ))
    }
}

fn config_path() -> Result<PathBuf, String> {
    let mut dir = dirs::config_dir().ok_or_else(|| "Could not find config directory".to_string())?;
    dir.push("RavenIsland");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    dir.push("license.json");
    Ok(dir)
}

fn random_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect()
}

fn hashed_device_id(source: &str) -> String {
    let digest = Sha256::digest(source.as_bytes());
    let hex = format!("{:x}", digest);
    format!("raven-{}", &hex[..40])
}

#[cfg(target_os = "windows")]
fn machine_fingerprint() -> Option<String> {
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .creation_flags(0x08000000)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let machine_guid = text
        .lines()
        .find(|line| line.contains("MachineGuid"))
        .and_then(|line| line.split_whitespace().last())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    Some(hashed_device_id(&format!("windows-machine-guid:{machine_guid}")))
}

#[cfg(not(target_os = "windows"))]
fn machine_fingerprint() -> Option<String> {
    None
}

fn make_device_id() -> String {
    machine_fingerprint().unwrap_or_else(|| format!("raven-{}", random_id()))
}

fn load_cache() -> Result<LicenseCache, String> {
    let path = config_path()?;
    let stable_device_id = make_device_id();
    if !path.exists() {
        let cache = LicenseCache {
            device_id: stable_device_id,
            status: "unknown".to_string(),
            ..LicenseCache::default()
        };
        save_cache(&cache)?;
        return Ok(cache);
    }

    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut cache: LicenseCache = serde_json::from_str(&raw).unwrap_or_default();
    if cache.device_id.trim().is_empty() || stable_device_id != cache.device_id {
        cache.device_id = stable_device_id;
        save_cache(&cache)?;
    }
    Ok(cache)
}

fn save_cache(cache: &LicenseCache) -> Result<(), String> {
    let path = config_path()?;
    let raw = serde_json::to_string_pretty(cache).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

fn parse_time(value: &Option<String>) -> Option<DateTime<Utc>> {
    value
        .as_deref()
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn cached_status(cache: &LicenseCache, message: Option<String>) -> LicenseStatus {
    LicenseStatus {
        status: cache.status.clone(),
        device_id: cache.device_id.clone(),
        license_key: cache.license_key.clone(),
        account_email: cache.account_email.clone(),
        account_name: cache.account_name.clone(),
        account_username: cache.account_username.clone(),
        account_picture: cache.account_picture.clone(),
        trial_expires_at: cache.trial_expires_at.clone(),
        message,
        force_trial_expired_preview: cache.force_trial_expired_preview,
    }
}

fn has_paid_grace(cache: &LicenseCache) -> bool {
    cache.status == "paid_active"
        && parse_time(&cache.last_checked_at)
            .map(|checked| Utc::now() - checked < Duration::days(OFFLINE_GRACE_DAYS))
            .unwrap_or(false)
}

fn has_trial_time(cache: &LicenseCache) -> bool {
    parse_time(&cache.trial_expires_at)
        .map(|expires| expires > Utc::now())
        .unwrap_or(false)
}

fn post_entitlement(path: &str, body: serde_json::Value) -> Result<ApiEntitlement, String> {
    let url = format!("{API_BASE_URL}{path}");
    match ureq::post(&url)
        .set("content-type", "application/json")
        .send_json(body)
    {
        Ok(response) => response
            .into_json::<ApiEntitlement>()
            .map_err(|e| e.to_string()),
        Err(ureq::Error::Status(_, response)) => {
            let entitlement = response
                .into_json::<ApiEntitlement>()
                .map_err(|e| e.to_string())?;
            Ok(entitlement)
        }
        Err(error) => Err(error.to_string()),
    }
}

fn post_account_status(token: &str, device_id: &str) -> Result<ApiAccountStatus, String> {
    let url = format!("{API_BASE_URL}/account/status");
    match ureq::post(&url)
        .set("content-type", "application/json")
        .set("authorization", &format!("Bearer {token}"))
        .send_json(serde_json::json!({ "deviceId": device_id }))
    {
        Ok(response) => response
            .into_json::<ApiAccountStatus>()
            .map_err(|e| e.to_string()),
        Err(ureq::Error::Status(_, response)) => response
            .into_json::<ApiAccountStatus>()
            .map_err(|e| e.to_string()),
        Err(error) => Err(error.to_string()),
    }
}
