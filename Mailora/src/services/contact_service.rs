use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::contact::{
    AddressEntry, Contact, ContactAddress, ContactEmail, ContactFull, ContactGroup,
    ContactPhone, ContactQuery, ContactRequest, ContactSocial, ContactSummary, EmailEntry,
    PhoneEntry,
};
use crate::pim::vcard::{parse_vcard, serialize_vcard};


/// Fetch list of contacts (summary view)
pub async fn list_contacts(pool: &SqlitePool, q: &ContactQuery) -> Result<Vec<ContactSummary>> {
    let limit = q.limit.unwrap_or(100).min(500);
    let offset = q.offset.unwrap_or(0);

    let mut sql = String::from(
        "SELECT c.id, c.full_name, c.first_name, c.last_name, c.company, c.photo_data, c.is_favorite, c.sync_status, \
         (SELECT email FROM contact_emails WHERE contact_id = c.id AND is_primary = 1 LIMIT 1) AS primary_email, \
         (SELECT phone FROM contact_phones WHERE contact_id = c.id AND is_primary = 1 LIMIT 1) AS primary_phone \
         FROM contacts c WHERE 1=1"
    );

    let mut binds: Vec<String> = Vec::new();

    if let Some(acc) = &q.account_id {
        sql.push_str(" AND c.account_id = ?");
        binds.push(acc.clone());
    }
    if let Some(fav) = q.favorite {
        if fav { sql.push_str(" AND c.is_favorite = 1"); }
    }
    if let Some(group_id) = &q.group_id {
        sql.push_str(" AND c.id IN (SELECT contact_id FROM contact_group_members WHERE group_id = ?)");
        binds.push(group_id.clone());
    }
    if let Some(search) = &q.q {
        if !search.is_empty() {
            // Use FTS if searching
            sql = format!(
                "SELECT c.id, c.full_name, c.first_name, c.last_name, c.company, c.photo_data, c.is_favorite, c.sync_status, \
                 (SELECT email FROM contact_emails WHERE contact_id = c.id AND is_primary = 1 LIMIT 1) AS primary_email, \
                 (SELECT phone FROM contact_phones WHERE contact_id = c.id AND is_primary = 1 LIMIT 1) AS primary_phone \
                 FROM contacts c \
                 JOIN contacts_fts fts ON c.id = fts.contact_id \
                 WHERE contacts_fts MATCH ? {} ORDER BY rank",
                if q.account_id.is_some() { "AND c.account_id = ?" } else { "" }
            );
            let fts_query = binds.drain(..).collect::<Vec<_>>();
            binds.push(format!("\"{}\"*", search.replace('"', "")));
            binds.extend(fts_query);
        }
    } else {
        let sort = q.sort.as_deref().unwrap_or("name_asc");
        match sort {
            "name_desc" => sql.push_str(" ORDER BY c.last_name DESC, c.first_name DESC"),
            "updated"   => sql.push_str(" ORDER BY c.updated_at DESC"),
            _           => sql.push_str(" ORDER BY c.last_name ASC, c.first_name ASC, c.full_name ASC"),
        }
    }

    sql.push_str(" LIMIT ? OFFSET ?");
    binds.push(limit.to_string());
    binds.push(offset.to_string());

    let mut qry = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, Option<String>, i64, String, Option<String>, Option<String>)>(&sql);
    for b in &binds {
        qry = qry.bind(b);
    }

    let rows = qry.fetch_all(pool).await?;
    Ok(rows.into_iter().map(|(id, full_name, first_name, last_name, company, photo_data, is_favorite, sync_status, primary_email, primary_phone)| {
        ContactSummary { id, full_name, first_name, last_name, company, primary_email, primary_phone, photo_data, is_favorite, sync_status }
    }).collect())
}

/// Fetch full contact with all sub-data
pub async fn get_contact(pool: &SqlitePool, id: &str) -> Result<Option<ContactFull>> {
    let contact = sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts WHERE id = ?"
    ).bind(id).fetch_optional(pool).await?;

    let Some(contact) = contact else { return Ok(None); };

    let emails = sqlx::query_as::<_, ContactEmail>(
        "SELECT * FROM contact_emails WHERE contact_id = ? ORDER BY is_primary DESC"
    ).bind(id).fetch_all(pool).await?;

    let phones = sqlx::query_as::<_, ContactPhone>(
        "SELECT * FROM contact_phones WHERE contact_id = ? ORDER BY is_primary DESC"
    ).bind(id).fetch_all(pool).await?;

    let addresses = sqlx::query_as::<_, ContactAddress>(
        "SELECT * FROM contact_addresses WHERE contact_id = ?"
    ).bind(id).fetch_all(pool).await?;

    let social = sqlx::query_as::<_, ContactSocial>(
        "SELECT * FROM contact_social WHERE contact_id = ?"
    ).bind(id).fetch_all(pool).await?;

    let groups: Vec<String> = sqlx::query_scalar(
        "SELECT group_id FROM contact_group_members WHERE contact_id = ?"
    ).bind(id).fetch_all(pool).await?;

    Ok(Some(ContactFull { contact, emails, phones, addresses, social, groups }))
}

/// Create a new contact
pub async fn create_contact(pool: &SqlitePool, req: ContactRequest) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO contacts (id, account_id, full_name, first_name, last_name, middle_name, prefix, suffix, \
         company, department, title, note, birthday, photo_data, website_url, gender, sync_status, created_at, updated_at) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,'pending_create',?,?)"
    )
    .bind(&id)
    .bind(&req.account_id)
    .bind(&req.full_name)
    .bind(&req.first_name)
    .bind(&req.last_name)
    .bind(&req.middle_name)
    .bind(&req.prefix)
    .bind(&req.suffix)
    .bind(&req.company)
    .bind(&req.department)
    .bind(&req.title)
    .bind(&req.note)
    .bind(&req.birthday)
    .bind(&req.photo_data)
    .bind(&req.website_url)
    .bind(&req.gender)
    .bind(&now)
    .bind(&now)
    .execute(pool).await?;

    // Insert sub-records
    upsert_sub_records(pool, &id, &req).await?;
    Ok(id)
}

/// Update an existing contact
pub async fn update_contact(pool: &SqlitePool, id: &str, req: ContactRequest) -> Result<bool> {
    let now = chrono::Utc::now().to_rfc3339();

    let rows = sqlx::query(
        "UPDATE contacts SET full_name=?, first_name=?, last_name=?, middle_name=?, prefix=?, suffix=?, \
         company=?, department=?, title=?, note=?, birthday=?, photo_data=?, website_url=?, gender=?, \
         sync_status=CASE WHEN sync_status='synced' THEN 'pending_update' ELSE sync_status END, \
         updated_at=? WHERE id=?"
    )
    .bind(&req.full_name).bind(&req.first_name).bind(&req.last_name)
    .bind(&req.middle_name).bind(&req.prefix).bind(&req.suffix)
    .bind(&req.company).bind(&req.department).bind(&req.title)
    .bind(&req.note).bind(&req.birthday).bind(&req.photo_data)
    .bind(&req.website_url).bind(&req.gender).bind(&now).bind(id)
    .execute(pool).await?;

    if rows.rows_affected() == 0 { return Ok(false); }

    // Replace sub-records
    sqlx::query("DELETE FROM contact_emails WHERE contact_id = ?").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM contact_phones WHERE contact_id = ?").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM contact_addresses WHERE contact_id = ?").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM contact_social WHERE contact_id = ?").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM contact_group_members WHERE contact_id = ?").bind(id).execute(pool).await?;

    upsert_sub_records(pool, id, &req).await?;
    Ok(true)
}

/// Mark contact for deletion (soft-delete, sync will propagate)
pub async fn delete_contact(pool: &SqlitePool, id: &str) -> Result<bool> {
    let rows = sqlx::query(
        "UPDATE contacts SET sync_status = 'pending_delete', updated_at = ? WHERE id = ?"
    )
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(id)
    .execute(pool).await?;
    Ok(rows.rows_affected() > 0)
}

/// Autocomplete suggestions for email `To:` field
pub async fn suggest_contacts(pool: &SqlitePool, query: &str, account_id: Option<&str>) -> Result<Vec<serde_json::Value>> {
    let q = format!("{}%", query.replace('"', ""));
    let sql = if account_id.is_some() {
        "SELECT c.full_name, e.email FROM contacts c \
         JOIN contact_emails e ON e.contact_id = c.id \
         WHERE (c.full_name LIKE ? OR e.email LIKE ?) AND c.account_id = ? \
         ORDER BY c.full_name LIMIT 10"
    } else {
        "SELECT c.full_name, e.email FROM contacts c \
         JOIN contact_emails e ON e.contact_id = c.id \
         WHERE c.full_name LIKE ? OR e.email LIKE ? \
         ORDER BY c.full_name LIMIT 10"
    };

    let mut qry = sqlx::query_as::<_, (String, String)>(sql)
        .bind(&q).bind(&q);
    if let Some(acc) = account_id { qry = qry.bind(acc); }

    let rows = qry.fetch_all(pool).await?;
    Ok(rows.into_iter()
        .map(|(name, email)| serde_json::json!({ "name": name, "email": email }))
        .collect())
}

// ── Groups ────────────────────────────────────────────────────

pub async fn list_groups(pool: &SqlitePool, account_id: &str) -> Result<Vec<ContactGroup>> {
    Ok(sqlx::query_as::<_, ContactGroup>(
        "SELECT * FROM contact_groups WHERE account_id = ? ORDER BY name"
    ).bind(account_id).fetch_all(pool).await?)
}

pub async fn create_group(pool: &SqlitePool, account_id: &str, name: &str, color: Option<&str>) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO contact_groups (id, account_id, name, color) VALUES (?,?,?,?)")
        .bind(&id).bind(account_id).bind(name).bind(color)
        .execute(pool).await?;
    Ok(id)
}

pub async fn add_to_group(pool: &SqlitePool, contact_id: &str, group_id: &str) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO contact_group_members (contact_id, group_id) VALUES (?,?)")
        .bind(contact_id).bind(group_id)
        .execute(pool).await?;
    Ok(())
}

pub async fn remove_from_group(pool: &SqlitePool, contact_id: &str, group_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM contact_group_members WHERE contact_id = ? AND group_id = ?")
        .bind(contact_id).bind(group_id)
        .execute(pool).await?;
    Ok(())
}

// ── vCard Import ──────────────────────────────────────────────

/// Import multiple vCards from a .vcf file content.
/// Returns (inserted, updated, skipped_count).
pub async fn import_vcf(pool: &SqlitePool, account_id: &str, vcf_content: &str) -> Result<(usize, usize, usize)> {
    let mut inserted = 0;
    let mut updated = 0;
    let mut skipped = 0;

    // Split into individual vCards
    let vcards: Vec<&str> = split_vcards(vcf_content);

    for raw in vcards {
        let Some(parsed) = parse_vcard(raw) else { skipped += 1; continue; };
        let full_name: String = match parsed.full_name.as_ref() {
            Some(n) => n.clone(),
            None => { skipped += 1; continue; }
        };

        // Check if UID already exists
        let existing_id: Option<String> = if let Some(uid) = &parsed.uid {
            sqlx::query_scalar("SELECT id FROM contacts WHERE vcard_uid = ? AND account_id = ?")
                .bind(uid).bind(account_id)
                .fetch_optional(pool).await?
        } else { None };

        let id = if let Some(eid) = existing_id {
            // Update
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                "UPDATE contacts SET full_name=?, first_name=?, last_name=?, company=?, title=?, note=?, birthday=?, raw_vcard=?, updated_at=? WHERE id=?"
            )
            .bind(&full_name).bind(&parsed.first_name).bind(&parsed.last_name)
            .bind(&parsed.company).bind(&parsed.title).bind(&parsed.note)
            .bind(&parsed.birthday).bind(raw).bind(&now).bind(&eid)
            .execute(pool).await?;
            updated += 1;
            eid
        } else {
            // Create new
            let req = ContactRequest {
                account_id: account_id.to_string(),
                full_name: full_name.clone(),
                first_name: parsed.first_name.clone(),
                last_name: parsed.last_name.clone(),
                middle_name: parsed.middle_name.clone(),
                prefix: parsed.prefix.clone(),
                suffix: parsed.suffix.clone(),
                company: parsed.company.clone(),
                department: parsed.department.clone(),
                title: parsed.title.clone(),
                note: parsed.note.clone(),
                birthday: parsed.birthday.clone(),
                photo_data: parsed.photo_data.clone(),
                website_url: parsed.website_url.clone(),
                gender: parsed.gender.clone(),
                emails: Some(parsed.emails.clone()),
                phones: Some(parsed.phones.clone()),
                addresses: Some(parsed.addresses.clone()),
                social: Some(parsed.social.clone()),
                group_ids: None,
            };
            let id = create_contact(pool, req).await?;
            // Set sync_status to 'local' (not pending_create for imports)
            sqlx::query("UPDATE contacts SET sync_status='local', vcard_uid=?, raw_vcard=? WHERE id=?")
                .bind(&parsed.uid).bind(raw).bind(&id)
                .execute(pool).await?;
            inserted += 1;
            id
        };

        // Handle categories → groups
        for cat in &parsed.categories {
            // Find or create group
            let gid: Option<String> = sqlx::query_scalar(
                "SELECT id FROM contact_groups WHERE account_id = ? AND name = ?"
            ).bind(account_id).bind(cat).fetch_optional(pool).await?;
            let gid = if let Some(g) = gid { g } else {
                create_group(pool, account_id, cat, None).await?
            };
            let _ = add_to_group(pool, &id, &gid).await;
        }
    }

    Ok((inserted, updated, skipped))
}

/// Export contacts to a .vcf string
pub async fn export_vcf(pool: &SqlitePool, account_id: &str, group_id: Option<&str>) -> Result<String> {

    let ids: Vec<String> = if let Some(gid) = group_id {
        sqlx::query_scalar(
            "SELECT c.id FROM contacts c JOIN contact_group_members m ON c.id = m.contact_id \
             WHERE c.account_id = ? AND m.group_id = ? AND c.sync_status != 'pending_delete'"
        ).bind(account_id).bind(gid).fetch_all(pool).await?
    } else {
        sqlx::query_scalar("SELECT id FROM contacts WHERE account_id = ? AND sync_status != 'pending_delete'")
            .bind(account_id).fetch_all(pool).await?
    };

    let mut output = String::new();
    for id in ids {
        let Some(full) = get_contact(pool, &id).await? else { continue; };
        let c = &full.contact;
        let uid = c.vcard_uid.as_deref().unwrap_or(&c.id);
        let emails_typed: Vec<EmailEntry> = full.emails.iter().map(|e| EmailEntry {
            email: e.email.clone(), label: e.label.clone(), is_primary: e.is_primary != 0
        }).collect();
        let phones_typed: Vec<PhoneEntry> = full.phones.iter().map(|p| PhoneEntry {
            phone: p.phone.clone(), label: p.label.clone(), is_primary: p.is_primary != 0
        }).collect();
        let addresses_typed: Vec<crate::models::contact::AddressEntry> = full.addresses.iter().map(|a| AddressEntry {
            label: a.label.clone(), street: a.street.clone(), city: a.city.clone(),
            region: a.region.clone(), postal_code: a.postal_code.clone(), country: a.country.clone()
        }).collect();
        let cats: Vec<String> = Vec::new(); // TODO: resolve group names
        let vcard = serialize_vcard(
            uid, &c.full_name,
            c.first_name.as_deref(), c.last_name.as_deref(), c.middle_name.as_deref(),
            c.prefix.as_deref(), c.suffix.as_deref(),
            c.company.as_deref(), c.department.as_deref(), c.title.as_deref(),
            c.note.as_deref(), c.birthday.as_deref(),
            &emails_typed, &phones_typed, &addresses_typed, &cats,
        );
        output.push_str(&vcard);
        output.push('\n');
    }
    Ok(output)
}

// ── Helpers ───────────────────────────────────────────────────

async fn upsert_sub_records(pool: &SqlitePool, contact_id: &str, req: &ContactRequest) -> Result<()> {
    if let Some(emails) = &req.emails {
        for e in emails {
            sqlx::query("INSERT INTO contact_emails (id, contact_id, email, label, is_primary) VALUES (?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(contact_id)
                .bind(&e.email).bind(&e.label).bind(e.is_primary as i64)
                .execute(pool).await?;
        }
    }
    if let Some(phones) = &req.phones {
        for p in phones {
            sqlx::query("INSERT INTO contact_phones (id, contact_id, phone, label, is_primary) VALUES (?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(contact_id)
                .bind(&p.phone).bind(&p.label).bind(p.is_primary as i64)
                .execute(pool).await?;
        }
    }
    if let Some(addresses) = &req.addresses {
        for a in addresses {
            sqlx::query("INSERT INTO contact_addresses (id, contact_id, label, street, city, region, postal_code, country) VALUES (?,?,?,?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(contact_id)
                .bind(&a.label).bind(&a.street).bind(&a.city)
                .bind(&a.region).bind(&a.postal_code).bind(&a.country)
                .execute(pool).await?;
        }
    }
    if let Some(social) = &req.social {
        for s in social {
            sqlx::query("INSERT INTO contact_social (id, contact_id, service, url) VALUES (?,?,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(contact_id)
                .bind(&s.service).bind(&s.url)
                .execute(pool).await?;
        }
    }
    if let Some(gids) = &req.group_ids {
        for gid in gids {
            let _ = add_to_group(pool, contact_id, gid).await;
        }
    }
    Ok(())
}

fn split_vcards(content: &str) -> Vec<&str> {
    let mut vcards = Vec::new();
    let mut start = 0;
    let mut pos = 0;
    while pos < content.len() {
        if content[pos..].starts_with("BEGIN:VCARD") {
            start = pos;
        } else if content[pos..].starts_with("END:VCARD") {
            let end = pos + "END:VCARD".len();
            vcards.push(&content[start..end]);
            pos = end;
            continue;
        }
        pos += 1;
    }
    vcards
}

// ── Conflicts & Duplicates ────────────────────────────────────

use crate::models::contact::{ResolveConflictRequest, MergeDuplicatesRequest};

pub async fn list_conflicts(pool: &SqlitePool, account_id: &str) -> Result<Vec<serde_json::Value>> {
    #[derive(sqlx::FromRow)]
    struct ConflictRowRaw {
        id: String,
        contact_id: String,
        remote_data: String,
    }

    let rows: Vec<ConflictRowRaw> = sqlx::query_as(
        r#"SELECT c.id, c.contact_id, c.remote_data
           FROM contact_conflicts c
           JOIN contacts cn ON c.contact_id = cn.id
           WHERE cn.account_id = ? AND c.resolved = 0"#
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;

    let mut res = Vec::new();
    for r in rows {
        let local_full = get_contact(pool, &r.contact_id).await?;
        let remote_parsed = crate::pim::vcard::parse_vcard(&r.remote_data);
        res.push(serde_json::json!({
            "id": r.id,
            "contact_id": r.contact_id,
            "local_data": local_full,
            "remote_data_vcard": r.remote_data,
            "remote_data_parsed": remote_parsed
        }));
    }
    Ok(res)
}

pub async fn resolve_conflict(
    pool: &SqlitePool,
    account_id: &str,
    conflict_id: &str,
    req: ResolveConflictRequest,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    #[derive(sqlx::FromRow)]
    struct ConflictRow {
        id: String,
        contact_id: String,
    }

    let conflict: Option<ConflictRow> = sqlx::query_as(
        "SELECT id, contact_id FROM contact_conflicts WHERE id = ?"
    )
    .bind(conflict_id)
    .fetch_optional(&mut *tx)
    .await?;
    let conflict = conflict.ok_or_else(|| anyhow::anyhow!("Conflict not found"))?;

    sqlx::query("UPDATE contact_conflicts SET resolved = 1 WHERE id = ?")
        .bind(conflict_id)
        .execute(&mut *tx)
        .await?;

    match req.resolution.as_str() {
        "local" => {
            sqlx::query("UPDATE contacts SET sync_status = 'pending_update' WHERE id = ?")
                .bind(&conflict.contact_id)
                .execute(&mut *tx)
                .await?;
        }
        "merge" => {
            if let Some(merged) = req.merged_data {
                tx.commit().await?; // commit before passing pool
                update_contact(pool, &conflict.contact_id, merged).await?;
                return Ok(());
            }
        }
        _ => {}
    }

    tx.commit().await?;
    Ok(())
}

pub async fn find_duplicates(pool: &SqlitePool, account_id: &str) -> Result<Vec<serde_json::Value>> {
    #[derive(sqlx::FromRow)]
    struct EmailDupRow {
        email: Option<String>,
        ids: Option<String>,
    }

    let email_dups: Vec<EmailDupRow> = sqlx::query_as(
        r#"SELECT e.email, GROUP_CONCAT(c.id) as ids 
           FROM contact_emails e
           JOIN contacts c ON e.contact_id = c.id
           WHERE c.account_id = ? AND c.sync_status != 'pending_delete'
           GROUP BY e.email HAVING COUNT(e.contact_id) > 1"#
    )
    .bind(account_id)
    .fetch_all(pool).await?;

    #[derive(sqlx::FromRow)]
    struct NameDupRow {
        full_name: Option<String>,
        ids: Option<String>,
    }

    let name_dups: Vec<NameDupRow> = sqlx::query_as(
        r#"SELECT full_name, GROUP_CONCAT(id) as ids
           FROM contacts 
           WHERE account_id = ? AND sync_status != 'pending_delete' AND full_name != 'Unknown'
           GROUP BY full_name HAVING COUNT(id) > 1"#
    )
    .bind(account_id)
    .fetch_all(pool).await?;

    let mut groups_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for d in email_dups {
        let ids: Vec<String> = d.ids.unwrap_or_default().split(',').map(String::from).collect();
        groups_map.insert(format!("email:{}", d.email.unwrap_or_default()), ids);
    }
    for d in name_dups {
        let ids: Vec<String> = d.ids.unwrap_or_default().split(',').map(String::from).collect();
        groups_map.insert(format!("name:{}", d.full_name.unwrap_or_default()), ids);
    }

    let mut res = Vec::new();
    for (reason, ids) in groups_map {
        let mut summaries = Vec::new();
        let mut unique_ids = ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        
        for id in unique_ids {
            if let Some(c) = get_contact(pool, &id).await? {
                summaries.push(c);
            }
        }
        if summaries.len() > 1 {
            res.push(serde_json::json!({
                "reason": reason,
                "contacts": summaries
            }));
        }
    }

    Ok(res)
}

pub async fn merge_duplicates(pool: &SqlitePool, account_id: &str, req: MergeDuplicatesRequest) -> Result<()> {
    // Delete target duplicates (the primary has theoretically already been combined via UI + PUT /contacts/:id)
    for tid in req.target_ids {
        if tid != req.primary_id {
            delete_contact(pool, &tid).await?;
        }
    }
    Ok(())
}
