use anyhow::Result;
use serde::{Deserialize, Serialize};
use aes_gcm::aead::Aead; // bring encrypt/decrypt trait into scope
use base64::Engine; // needed for STANDARD.decode

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EmailProvider {
    Gmail,
    Outlook,
    Yahoo,
    Icloud,
    #[default]
    Custom,
}

impl EmailProvider {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "gmail" => Self::Gmail,
            "outlook" => Self::Outlook,
            "yahoo" => Self::Yahoo,
            "icloud" => Self::Icloud,
            _ => Self::Custom,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Gmail => "gmail",
            Self::Outlook => "outlook",
            Self::Yahoo => "yahoo",
            Self::Icloud => "icloud",
            Self::Custom => "custom",
        }
    }

    /// Get default IMAP/SMTP settings for known providers
    pub fn default_config(&self) -> ProviderConfig {
        match self {
            Self::Gmail => ProviderConfig {
                imap_host: "imap.gmail.com".to_string(),
                imap_port: 993,
                smtp_host: "smtp.gmail.com".to_string(),
                smtp_port: 587,
                carddav_url: Some("https://www.googleapis.com/carddav/v1/principals/[EMAIL]/lists/default/".to_string()),
                caldav_url: Some("https://apidata.googleusercontent.com/caldav/v2/[EMAIL]/events".to_string()),
            },
            Self::Outlook => ProviderConfig {
                imap_host: "outlook.office365.com".to_string(),
                imap_port: 993,
                smtp_host: "smtp.office365.com".to_string(),
                smtp_port: 587,
                carddav_url: None,
                caldav_url: None,
            },
            Self::Yahoo => ProviderConfig {
                imap_host: "imap.mail.yahoo.com".to_string(),
                imap_port: 993,
                smtp_host: "smtp.mail.yahoo.com".to_string(),
                smtp_port: 587,
                carddav_url: Some("https://carddav.address.yahoo.com/".to_string()),
                caldav_url: Some("https://caldav.calendar.yahoo.com/".to_string()),
            },
            Self::Icloud => ProviderConfig {
                imap_host: "imap.mail.me.com".to_string(),
                imap_port: 993,
                smtp_host: "smtp.mail.me.com".to_string(),
                smtp_port: 587,
                carddav_url: Some("https://contacts.icloud.com/".to_string()),
                caldav_url: Some("https://caldav.icloud.com/".to_string()),
            },
            Self::Custom => ProviderConfig {
                imap_host: String::new(),
                imap_port: 993,
                smtp_host: String::new(),
                smtp_port: 587,
                carddav_url: None,
                caldav_url: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub imap_host: String,
    pub imap_port: u16,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub carddav_url: Option<String>,
    pub caldav_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Account {
    pub id: String,
    pub email: String,
    #[sqlx(skip)]
    pub provider: EmailProvider,
    pub display_name: Option<String>,
    pub imap_host: String,
    pub imap_port: u16,
    pub smtp_host: String,
    pub smtp_port: u16,
    #[serde(skip_serializing)]
    pub credentials_encrypted: String, // Base64 encoded "email:password"
    pub enabled: bool,
    pub sync_frequency_secs: i64,
    pub last_sync_ts: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    // New per-account send behavior hints
    pub append_policy: Option<String>, // stored as text column (auto|never|force)
    pub sent_folder_hint: Option<String>,
    pub color: Option<String>,
    pub carddav_url: Option<String>,
    pub caldav_url: Option<String>,
    // Helper field for password (populated from credentials_encrypted)
    #[sqlx(skip)]
    #[serde(skip)]
    pub password: String,
}

impl Account {
    /// Load account and decrypt password
    pub fn with_password(mut self) -> Result<Self> {
        let (_, password) = Self::decode_credentials(&self.credentials_encrypted)?;
        self.password = password;
        Ok(self)
    }
}

impl Account {
    /// Generate account ID from email
    pub fn generate_id(email: &str) -> String {
        format!("acc_{}", email.replace('@', "_").replace('.', "_"))
    }

    fn key_from_env() -> Option<[u8; 32]> {
        if let Ok(key_b64) = std::env::var("MAILORA_KEY") {
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(key_b64) {
                if bytes.len() == 32 {
                    let mut k = [0u8;32]; k.copy_from_slice(&bytes); return Some(k);
                }
            }
        }
        None
    }
    fn encrypt(creds: &str) -> String {
        if let Some(key) = Self::key_from_env() {
            use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
            use rand::RngCore;
            let cipher = Aes256Gcm::new((&key).into());
            let mut nonce_bytes = [0u8;12]; rand::thread_rng().fill_bytes(&mut nonce_bytes);
            let nonce = Nonce::from_slice(&nonce_bytes);
            if let Ok(ct) = cipher.encrypt(nonce, creds.as_bytes()) {
                let mut blob = Vec::with_capacity(1+12+ct.len());
                blob.push(1u8); // v1
                blob.extend_from_slice(&nonce_bytes);
                blob.extend_from_slice(&ct);
                use base64::Engine; return base64::engine::general_purpose::STANDARD.encode(&blob);
            }
        }
        // fallback base64
        use base64::Engine; base64::engine::general_purpose::STANDARD.encode(creds.as_bytes())
    }
    fn decrypt(s: &str) -> Result<String> {
        use base64::Engine; let bytes = base64::engine::general_purpose::STANDARD.decode(s)?;
        if bytes.first() == Some(&1u8) && bytes.len()>13 { // v1|nonce|ct
            if let Some(key) = Self::key_from_env() {
                let (v, rest) = bytes.split_first().unwrap(); let _ = v;
                let (nonce_bytes, ct) = rest.split_at(12);
                use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
                let cipher = Aes256Gcm::new((&key).into());
                let nonce = Nonce::from_slice(nonce_bytes);
                if let Ok(pt) = cipher.decrypt(nonce, ct) { return Ok(String::from_utf8(pt)?); }
            }
        }
        Ok(String::from_utf8(bytes)?)
    }
    /// Encode credentials (AES-GCM with MAILORA_KEY; fallback base64)
    pub fn encode_credentials(email: &str, password: &str) -> String {
        let creds = format!("{}:{}", email, password);
        Self::encrypt(&creds)
    }

    /// Decode credentials
    pub fn decode_credentials(encoded: &str) -> Result<(String, String)> {
        let creds = Self::decrypt(encoded)?;
        let parts: Vec<&str> = creds.splitn(2, ':').collect();
        if parts.len() != 2 { anyhow::bail!("Invalid credentials format"); }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Get credentials for this account
    pub fn get_credentials(&self) -> Result<(String, String)> {
        Self::decode_credentials(&self.credentials_encrypted)
    }

    pub fn append_policy_enum(&self) -> AppendPolicy {
        self.append_policy
            .as_ref()
            .map(|s| AppendPolicy::from_str(s))
            .unwrap_or(AppendPolicy::Auto)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppendPolicy {
    Auto,   // search-first then maybe APPEND (default)
    Never,  // never APPEND (only rely on provider auto-sent copy)
    Force,  // always APPEND regardless of provider
}

impl Default for AppendPolicy {
    fn default() -> Self {
        AppendPolicy::Auto
    }
}

impl AppendPolicy {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "auto" => Self::Auto,
            "never" => Self::Never,
            "force" => Self::Force,
            _ => Self::Auto,
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Auto => "auto",
            Self::Never => "never",
            Self::Force => "force",
        }
    }
}

// Database mapping helpers
impl Account {
    pub fn provider_str(&self) -> String {
        self.provider.as_str().to_string()
    }
}
