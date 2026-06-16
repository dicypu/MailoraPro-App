/// vCard 3.0 / 4.0 parser and serializer
/// Handles common properties: FN, N, EMAIL, TEL, ADR, ORG, TITLE, BDAY, NOTE, PHOTO, UID, REV, URL, CATEGORIES
use crate::models::contact::{AddressEntry, EmailEntry, PhoneEntry, SocialEntry};

#[derive(Debug, Default, serde::Serialize)]
pub struct ParsedVCard {
    pub uid: Option<String>,
    pub full_name: Option<String>,
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
    pub emails: Vec<EmailEntry>,
    pub phones: Vec<PhoneEntry>,
    pub addresses: Vec<AddressEntry>,
    pub social: Vec<SocialEntry>,
    pub categories: Vec<String>, // Group names
    pub rev: Option<String>,     // Last modified timestamp
}

/// Parse a single vCard string (BEGIN:VCARD ... END:VCARD).
/// Handles line folding (CRLF + whitespace continuation).
pub fn parse_vcard(raw: &str) -> Option<ParsedVCard> {
    let unfolded = unfold_lines(raw);
    let mut card = ParsedVCard::default();
    let mut in_vcard = false;

    for line in unfolded.lines() {
        let line = line.trim_end();
        if line.eq_ignore_ascii_case("BEGIN:VCARD") {
            in_vcard = true;
            continue;
        }
        if line.eq_ignore_ascii_case("END:VCARD") {
            break;
        }
        if !in_vcard { continue; }
        if line.is_empty() { continue; }

        // Split property name (with params) from value
        let (prop_raw, value) = match line.splitn(2, ':').collect::<Vec<_>>()[..] {
            [p, v] => (p.to_string(), v.to_string()),
            _ => continue,
        };

        // Extract base property name and params
        let parts: Vec<&str> = prop_raw.splitn(2, ';').collect();
        let prop_name = parts[0].to_uppercase();
        let params = if parts.len() > 1 { parts[1] } else { "" };

        // Extract TYPE from params (e.g., TYPE=WORK,INTERNET or TYPE=HOME)
        let type_label = extract_type_param(params);
        let value = decode_value(&value);

        match prop_name.as_str() {
            "UID" => card.uid = Some(value),
            "REV" => card.rev = Some(value),
            "FN" => card.full_name = Some(value),
            "N" => {
                // N:Last;First;Middle;Prefix;Suffix
                let parts: Vec<&str> = value.splitn(5, ';').collect();
                card.last_name   = parts.get(0).map(|s| s.to_string()).filter(|s| !s.is_empty());
                card.first_name  = parts.get(1).map(|s| s.to_string()).filter(|s| !s.is_empty());
                card.middle_name = parts.get(2).map(|s| s.to_string()).filter(|s| !s.is_empty());
                card.prefix      = parts.get(3).map(|s| s.to_string()).filter(|s| !s.is_empty());
                card.suffix      = parts.get(4).map(|s| s.to_string()).filter(|s| !s.is_empty());
            }
            "EMAIL" => {
                let label = normalize_email_label(&type_label);
                let is_primary = card.emails.is_empty(); // first email is primary
                card.emails.push(EmailEntry { email: value, label, is_primary });
            }
            "TEL" => {
                let label = normalize_phone_label(&type_label);
                let is_primary = card.phones.is_empty();
                card.phones.push(PhoneEntry { phone: value, label, is_primary });
            }
            "ADR" => {
                // ADR:pobox;extended;street;city;region;postal;country
                let parts: Vec<&str> = value.splitn(7, ';').collect();
                let label = normalize_address_label(&type_label);
                card.addresses.push(AddressEntry {
                    label,
                    street:      parts.get(2).map(|s| s.to_string()).filter(|s| !s.is_empty()),
                    city:        parts.get(3).map(|s| s.to_string()).filter(|s| !s.is_empty()),
                    region:      parts.get(4).map(|s| s.to_string()).filter(|s| !s.is_empty()),
                    postal_code: parts.get(5).map(|s| s.to_string()).filter(|s| !s.is_empty()),
                    country:     parts.get(6).map(|s| s.to_string()).filter(|s| !s.is_empty()),
                });
            }
            "ORG" => {
                // ORG:Company;Department
                let parts: Vec<&str> = value.splitn(2, ';').collect();
                card.company    = parts.get(0).map(|s| s.to_string()).filter(|s| !s.is_empty());
                card.department = parts.get(1).map(|s| s.to_string()).filter(|s| !s.is_empty());
            }
            "TITLE" => card.title = Some(value),
            "NOTE"  => card.note  = Some(value),
            "BDAY"  => card.birthday = Some(normalize_date(&value)),
            "PHOTO" => card.photo_data = Some(value),
            "URL"   => card.website_url = Some(value),
            "GENDER" => card.gender = Some(value),
            "LANG"   => card.language = Some(value),
            "TZ"     => card.timezone = Some(value),
            "CATEGORIES" => {
                // CATEGORIES:Friends,Work
                for cat in value.split(',') {
                    let c = cat.trim().to_string();
                    if !c.is_empty() { card.categories.push(c); }
                }
            }
            name if name.starts_with("X-SOCIALPROFILE") || name.starts_with("X-") => {
                // Try to detect social network from type param
                let service = extract_social_service(name, params);
                if !value.is_empty() {
                    card.social.push(SocialEntry { service, url: value });
                }
            }
            _ => {} // Unknown property — ignored (raw_vcard stores original)
        }
    }

    // Fallback: if no FN but have N, compose it
    if card.full_name.is_none() {
        let parts = [card.prefix.as_deref(), card.first_name.as_deref(), card.middle_name.as_deref(), card.last_name.as_deref(), card.suffix.as_deref()];
        let composed: Vec<&str> = parts.iter().filter_map(|p| *p).collect();
        if !composed.is_empty() {
            card.full_name = Some(composed.join(" "));
        }
    }

    card.full_name.as_ref()?; // Must have a displayable name
    Some(card)
}

/// Serialize a contact back to vCard 3.0 format
pub fn serialize_vcard(
    uid: &str,
    full_name: &str,
    first_name: Option<&str>,
    last_name: Option<&str>,
    middle_name: Option<&str>,
    prefix: Option<&str>,
    suffix: Option<&str>,
    company: Option<&str>,
    department: Option<&str>,
    title: Option<&str>,
    note: Option<&str>,
    birthday: Option<&str>,
    emails: &[EmailEntry],
    phones: &[PhoneEntry],
    addresses: &[AddressEntry],
    categories: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("BEGIN:VCARD\r\nVERSION:3.0\r\n");
    out.push_str(&format!("UID:{}\r\n", uid));
    out.push_str(&format!("FN:{}\r\n", fold_value(full_name)));

    // N property
    let n = format!(
        "N:{};{};{};{};{}\r\n",
        last_name.unwrap_or(""),
        first_name.unwrap_or(""),
        middle_name.unwrap_or(""),
        prefix.unwrap_or(""),
        suffix.unwrap_or("")
    );
    out.push_str(&n);

    if let Some(c) = company {
        let dept = department.unwrap_or("");
        out.push_str(&format!("ORG:{};{}\r\n", c, dept));
    }
    if let Some(t) = title   { out.push_str(&format!("TITLE:{}\r\n", t)); }
    if let Some(b) = birthday { out.push_str(&format!("BDAY:{}\r\n", b)); }
    if let Some(n) = note    { out.push_str(&format!("NOTE:{}\r\n", fold_value(n))); }

    for e in emails {
        let label = vcard_email_type(&e.label);
        out.push_str(&format!("EMAIL;TYPE={}:{}\r\n", label, e.email));
    }
    for p in phones {
        let label = vcard_phone_type(&p.label);
        out.push_str(&format!("TEL;TYPE={}:{}\r\n", label, p.phone));
    }
    for a in addresses {
        let label = vcard_address_type(&a.label);
        out.push_str(&format!(
            "ADR;TYPE={}:;;{};{};{};{};{}\r\n",
            label,
            a.street.as_deref().unwrap_or(""),
            a.city.as_deref().unwrap_or(""),
            a.region.as_deref().unwrap_or(""),
            a.postal_code.as_deref().unwrap_or(""),
            a.country.as_deref().unwrap_or("")
        ));
    }
    if !categories.is_empty() {
        out.push_str(&format!("CATEGORIES:{}\r\n", categories.join(",")));
    }

    let rev = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    out.push_str(&format!("REV:{}\r\n", rev));
    out.push_str("END:VCARD\r\n");
    out
}

// ── Internal helpers ─────────────────────────────────────────

/// Unfold RFC 2425 folded lines (CRLF + whitespace = continuation)
fn unfold_lines(input: &str) -> String {
    input
        .replace("\r\n ", "")
        .replace("\r\n\t", "")
        .replace("\n ", "")
        .replace("\n\t", "")
}

/// Extract TYPE param value from the params string
fn extract_type_param(params: &str) -> String {
    for part in params.split(';') {
        let kv: Vec<&str> = part.splitn(2, '=').collect();
        if kv.len() == 2 && kv[0].trim().to_uppercase() == "TYPE" {
            return kv[1].trim().to_uppercase();
        }
    }
    String::new()
}

/// Decode common vCard value escapes (\n, \,, \\)
fn decode_value(val: &str) -> String {
    val.replace("\\n", "\n")
       .replace("\\N", "\n")
       .replace("\\,", ",")
       .replace("\\;", ";")
       .replace("\\\\", "\\")
}

/// Normalize birthday to YYYY-MM-DD
fn normalize_date(val: &str) -> String {
    let d = val.trim().replace("--", "");
    if d.len() == 8 && d.chars().all(|c| c.is_ascii_digit()) {
        // YYYYMMDD → YYYY-MM-DD
        format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..8])
    } else {
        d
    }
}

fn normalize_email_label(t: &str) -> String {
    if t.contains("WORK") { "work".into() }
    else if t.contains("HOME") { "home".into() }
    else { "other".into() }
}

fn normalize_phone_label(t: &str) -> String {
    if t.contains("CELL") || t.contains("MOBILE") { "mobile".into() }
    else if t.contains("WORK") { "work".into() }
    else if t.contains("HOME") { "home".into() }
    else if t.contains("FAX") { "fax".into() }
    else { "other".into() }
}

fn normalize_address_label(t: &str) -> String {
    if t.contains("WORK") { "work".into() }
    else if t.contains("HOME") { "home".into() }
    else { "other".into() }
}

fn extract_social_service(prop_name: &str, params: &str) -> String {
    // X-SOCIALPROFILE;type=linkedin → "linkedin"
    for part in params.split(';') {
        let kv: Vec<&str> = part.splitn(2, '=').collect();
        if kv.len() == 2 && kv[0].trim().to_lowercase() == "type" {
            return kv[1].trim().to_lowercase();
        }
    }
    // Fallback: parse from X-LINKEDIN, X-TWITTER, etc.
    prop_name.trim_start_matches("X-").to_lowercase()
}

/// Fold long vCard values at 75 bytes
fn fold_value(val: &str) -> String {
    if val.len() <= 75 { return val.to_string(); }
    let mut out = String::new();
    let mut count = 0;
    for ch in val.chars() {
        if count > 70 {
            out.push_str("\r\n ");
            count = 1;
        }
        out.push(ch);
        count += ch.len_utf8();
    }
    out
}

fn vcard_email_type(label: &str) -> &'static str {
    match label { "work" => "WORK,INTERNET", "home" => "HOME,INTERNET", _ => "INTERNET" }
}
fn vcard_phone_type(label: &str) -> &'static str {
    match label { "mobile" => "CELL", "work" => "WORK", "home" => "HOME", "fax" => "FAX", _ => "VOICE" }
}
fn vcard_address_type(label: &str) -> &'static str {
    match label { "work" => "WORK", "home" => "HOME", _ => "POSTAL" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_vcard() {
        let raw = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Ahmet Yılmaz\r\nN:Yılmaz;Ahmet;;;\r\nEMAIL;TYPE=WORK,INTERNET:ahmet@ornek.com\r\nTEL;TYPE=CELL:+905551234567\r\nEND:VCARD\r\n";
        let card = parse_vcard(raw).expect("should parse");
        assert_eq!(card.full_name.as_deref(), Some("Ahmet Yılmaz"));
        assert_eq!(card.first_name.as_deref(), Some("Ahmet"));
        assert_eq!(card.last_name.as_deref(), Some("Yılmaz"));
        assert_eq!(card.emails.len(), 1);
        assert_eq!(card.emails[0].label, "work");
        assert_eq!(card.phones.len(), 1);
        assert_eq!(card.phones[0].label, "mobile");
    }

    #[test]
    fn test_parse_org() {
        let raw = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Test User\r\nORG:Ornek AS;IT Departmani\r\nEND:VCARD\r\n";
        let card = parse_vcard(raw).unwrap();
        assert_eq!(card.company.as_deref(), Some("Ornek AS"));
        assert_eq!(card.department.as_deref(), Some("IT Departmani"));
    }

    #[test]
    fn test_parse_line_folding() {
        let raw = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Very Long\r\n  Name Here\r\nEND:VCARD\r\n";
        let card = parse_vcard(raw).unwrap();
        assert!(card.full_name.is_some());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let emails = vec![EmailEntry { email: "test@example.com".into(), label: "work".into(), is_primary: true }];
        let phones = vec![PhoneEntry { phone: "+901234567".into(), label: "mobile".into(), is_primary: true }];
        let vcard = serialize_vcard("uid-123", "Test User", Some("Test"), Some("User"), None, None, None, None, None, None, None, None, &emails, &phones, &[], &[]);
        assert!(vcard.contains("BEGIN:VCARD"));
        assert!(vcard.contains("FN:Test User"));
        assert!(vcard.contains("test@example.com"));
        assert!(vcard.contains("END:VCARD"));
        // Should re-parse
        let reparsed = parse_vcard(&vcard).unwrap();
        assert_eq!(reparsed.full_name.as_deref(), Some("Test User"));
    }
}
