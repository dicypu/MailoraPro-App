use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Contact {
    pub id: String,
    pub account_id: String,
    pub vcard_uid: Option<String>,
    pub etag: Option<String>,
    pub href: Option<String>,
    pub full_name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub company: Option<String>,
    pub department: Option<String>,
    pub title: Option<String>,
    pub note: Option<String>,
    pub birthday: Option<String>,
    pub photo_data: Option<String>,
    pub website_url: Option<String>,
    pub gender: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub is_favorite: i64,
    pub sync_status: String, // 'local' | 'synced' | 'conflict' | 'pending_delete' | 'pending_create' | 'pending_update'
    pub raw_vcard: Option<String>,
    pub synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContactEmail {
    pub id: String,
    pub contact_id: String,
    pub email: String,
    pub label: String,
    pub is_primary: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContactPhone {
    pub id: String,
    pub contact_id: String,
    pub phone: String,
    pub label: String,
    pub is_primary: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContactAddress {
    pub id: String,
    pub contact_id: String,
    pub label: String,
    pub street: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContactSocial {
    pub id: String,
    pub contact_id: String,
    pub service: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContactGroup {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub color: Option<String>,
    pub vcard_kind: Option<String>,
}

/// Full contact with all related data — returned by detail API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactFull {
    #[serde(flatten)]
    pub contact: Contact,
    pub emails: Vec<ContactEmail>,
    pub phones: Vec<ContactPhone>,
    pub addresses: Vec<ContactAddress>,
    pub social: Vec<ContactSocial>,
    pub groups: Vec<String>, // group IDs
}

/// Lightweight contact — returned by list API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactSummary {
    pub id: String,
    pub full_name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company: Option<String>,
    pub primary_email: Option<String>,
    pub primary_phone: Option<String>,
    pub photo_data: Option<String>,
    pub is_favorite: i64,
    pub sync_status: String,
}

/// Request body for creating/updating a contact
#[derive(Debug, Deserialize)]
pub struct ContactRequest {
    pub account_id: String,
    pub full_name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub company: Option<String>,
    pub department: Option<String>,
    pub title: Option<String>,
    pub note: Option<String>,
    pub birthday: Option<String>,
    pub photo_data: Option<String>,
    pub website_url: Option<String>,
    pub gender: Option<String>,
    pub emails: Option<Vec<EmailEntry>>,
    pub phones: Option<Vec<PhoneEntry>>,
    pub addresses: Option<Vec<AddressEntry>>,
    pub social: Option<Vec<SocialEntry>>,
    pub group_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmailEntry {
    pub email: String,
    pub label: String,
    pub is_primary: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PhoneEntry {
    pub phone: String,
    pub label: String,
    pub is_primary: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AddressEntry {
    pub label: String,
    pub street: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SocialEntry {
    pub service: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct ContactQuery {
    pub q: Option<String>,
    pub account_id: Option<String>,
    pub group_id: Option<String>,
    pub favorite: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort: Option<String>, // 'name_asc' | 'name_desc' | 'updated'
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ContactConflict {
    pub id: String,
    pub contact_id: String,
    pub local_data: String,
    pub remote_data: String,
    pub resolved: i64,
}

#[derive(Debug, Deserialize)]
pub struct ResolveConflictRequest {
    pub resolution: String, // 'local', 'remote', or 'merge'
    #[serde(default)]
    pub merged_data: Option<ContactRequest>, // Only used for 'merge'
}

#[derive(Debug, Deserialize)]
pub struct MergeDuplicatesRequest {
    pub account_id: String,
    pub primary_id: String,
    pub target_ids: Vec<String>,
}
