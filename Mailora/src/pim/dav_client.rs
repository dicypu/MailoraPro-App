/// Minimal async WebDAV HTTP client for CardDAV and CalDAV operations.
/// Supports: PROPFIND, REPORT, GET, PUT, DELETE
use anyhow::{anyhow, Result};
use reqwest::{Client, Method, StatusCode};

pub struct DavClient {
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

#[derive(Debug, Clone)]
pub struct DavResource {
    pub href: String,
    pub etag: Option<String>,
    pub content_type: Option<String>,
}

impl DavClient {
    /// Construct a new DAV client.
    pub fn new(base_url: &str, username: &str, password: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    /// PROPFIND — list children of a collection.
    /// Returns a list of href strings.
    pub async fn propfind(&self, path: &str, depth: &str) -> Result<Vec<DavResource>> {
        let url = self.url(path);
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:">
  <prop>
    <getcontenttype/>
    <getetag/>
    <resourcetype/>
    <displayname/>
  </prop>
</propfind>"#;

        let resp = self.client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", depth)
            .header("Content-Type", "application/xml")
            .body(body)
            .send()
            .await?;

        if !resp.status().is_success() && resp.status() != StatusCode::from_u16(207).unwrap() {
            return Err(anyhow!("PROPFIND failed: {} {}", resp.status(), url));
        }

        let text = resp.text().await?;
        Ok(parse_multistatus_hrefs(&text, self.base_url_stripped()))
    }

    /// PROPFIND — list calendar collections (like calendars, INBOX, OUTBOX).
    pub async fn discover_calendars(&self, principal_url: &str) -> Result<Vec<DavResource>> {
        let url = self.url(principal_url);
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav">
  <prop>
    <resourcetype/>
    <displayname/>
    <cal:calendar-description/>
    <cal:supported-calendar-component-set/>
    <getctag xmlns="http://calendarserver.org/ns/"/>
    <sync-token/>
  </prop>
</propfind>"#;

        let resp = self.client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .body(body)
            .send()
            .await?;

        if !resp.status().is_success() && resp.status() != StatusCode::from_u16(207).unwrap() {
            return Err(anyhow!("PROPFIND calendars failed: {} {}", resp.status(), url));
        }

        let text = resp.text().await?;
        Ok(parse_multistatus_collections(&text, self.base_url_stripped()))
    }

    /// REPORT — fetch all card/cal hrefs with etags (CardDAV/CalDAV report).
    pub async fn report_addressbook(&self, collection_path: &str) -> Result<Vec<DavResource>> {
        let url = self.url(collection_path);
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<card:addressbook-query xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <d:getetag/>
  </d:prop>
  <card:filter/>
</card:addressbook-query>"#;

        let resp = self.client
            .request(Method::from_bytes(b"REPORT").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .body(body)
            .send()
            .await?;

        let text = resp.text().await?;
        Ok(parse_multistatus_hrefs(&text, self.base_url_stripped()))
    }

    /// REPORT — fetch all calendar hrefs with etags (CalDAV report).
    pub async fn report_calendar(&self, collection_path: &str) -> Result<Vec<DavResource>> {
        let url = self.url(collection_path);
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<cal:calendar-query xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag/>
  </d:prop>
  <cal:filter>
    <cal:comp-filter name="VCALENDAR">
      <cal:comp-filter name="VEVENT"/>
    </cal:comp-filter>
  </cal:filter>
</cal:calendar-query>"#;

        let resp = self.client
            .request(Method::from_bytes(b"REPORT").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .body(body)
            .send()
            .await?;

        let text = resp.text().await?;
        Ok(parse_multistatus_hrefs(&text, self.base_url_stripped()))
    }

    /// GET a single resource (vCard, iCal).
    pub async fn get(&self, path: &str) -> Result<(String, Option<String>)> {
        let url = self.url(path);
        let resp = self.client
            .get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("GET failed: {} {}", resp.status(), url));
        }

        let etag = resp.headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim_matches('"').to_string());

        let body = resp.text().await?;
        Ok((body, etag))
    }

    /// PUT a resource (create or update). Returns the new ETag.
    /// If `if_none_match` is true, uses `If-None-Match: *` (create new).
    /// Otherwise uses `If-Match: {etag}` to detect conflicts.
    pub async fn put(&self, path: &str, body: &str, content_type: &str, existing_etag: Option<&str>) -> Result<Option<String>> {
        let url = self.url(path);
        let mut req = self.client
            .put(&url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", content_type)
            .body(body.to_string());

        if let Some(etag) = existing_etag {
            req = req.header("If-Match", format!("\"{}\"", etag));
        } else {
            req = req.header("If-None-Match", "*");
        }

        let resp = req.send().await?;

        match resp.status().as_u16() {
            201 | 204 | 200 => {
                let etag = resp.headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.trim_matches('"').to_string());
                Ok(etag)
            }
            412 => Err(anyhow!("CONFLICT: ETag mismatch on PUT {}", url)),
            code => Err(anyhow!("PUT failed: {} {}", code, url)),
        }
    }

    /// DELETE a resource.
    pub async fn delete(&self, path: &str, etag: Option<&str>) -> Result<()> {
        let url = self.url(path);
        let mut req = self.client
            .delete(&url)
            .basic_auth(&self.username, Some(&self.password));

        if let Some(e) = etag {
            req = req.header("If-Match", format!("\"{}\"", e));
        }

        let resp = req.send().await?;
        if resp.status().is_success() || resp.status() == StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(anyhow!("DELETE failed: {} {}", resp.status(), url))
        }
    }

    /// Try to discover the principal URL and address book / calendar home.
    pub async fn discover_principal(&self) -> Result<String> {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:">
  <prop><current-user-principal/></prop>
</propfind>"#;

        let resp = self.client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &format!("{}/", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "0")
            .header("Content-Type", "application/xml")
            .body(body)
            .send()
            .await?;

        let text = resp.text().await?;
        // Simple XML extraction for current-user-principal
        let principal_href = match extract_xml_value(&text, "href")
            .or_else(|| extract_xml_value(&text, "d:href")) {
                Some(href) => href,
                None => {
                    tracing::warn!("discover_principal response: {}", text);
                    return Err(anyhow!("Could not discover principal href"));
                }
            };

        // Try to discover calendar-home-set
        let body_home = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav">
  <prop><cal:calendar-home-set/></prop>
</propfind>"#;
        
        let url = self.url(&principal_href);
        let resp2 = self.client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "0")
            .header("Content-Type", "application/xml")
            .body(body_home)
            .send()
            .await?;
            
        if resp2.status().is_success() || resp2.status() == StatusCode::from_u16(207).unwrap() {
             let text2 = resp2.text().await?;
             if let Some(home_set) = extract_xml_value(&text2, "href") {
                  return Ok(home_set);
             }
        }
        
        // Fallback to principal URL
        Ok(principal_href)
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        }
    }

    fn base_url_stripped(&self) -> &str {
        // Return just the scheme+host for relative href resolution
        if let Some(idx) = self.base_url.find("://") {
            if let Some(slash) = self.base_url[idx + 3..].find('/') {
                return &self.base_url[..idx + 3 + slash];
            }
        }
        &self.base_url
    }
}

/// Parse WebDAV multistatus XML response, extract hrefs and etags.
fn parse_multistatus_hrefs(xml: &str, base_url: &str) -> Vec<DavResource> {
    let mut resources = Vec::new();
    // Simple regex-free XML parsing: find <response> blocks
    let mut pos = 0;
    while let Some(start) = xml[pos..].find("<response>").or_else(|| xml[pos..].find("<d:response>")) {
        let block_start = pos + start;
        let end_tag = if xml[block_start..].contains("</response>") { "</response>" } else { "</d:response>" };
        let Some(block_end) = xml[block_start..].find(end_tag) else { break; };
        let block = &xml[block_start..block_start + block_end + end_tag.len()];
        pos = block_start + block_end + end_tag.len();

        let href = extract_xml_value(block, "href").unwrap_or_default();
        if href.is_empty() || href.ends_with('/') {
            continue; // Skip collections
        }
        let etag = extract_xml_value(block, "getetag")
            .map(|e| e.trim_matches('"').to_string());
        let content_type = extract_xml_value(block, "getcontenttype");

        // Make href absolute if relative
        let full_href = if href.starts_with('/') {
            format!("{}{}", base_url, href)
        } else if href.starts_with("http") {
            href
        } else {
            continue;
        };

        resources.push(DavResource { href: full_href, etag, content_type });
    }
    resources
}

fn parse_multistatus_collections(xml: &str, base_url: &str) -> Vec<DavResource> {
    let mut resources = Vec::new();
    let mut pos = 0;
    while let Some(start) = xml[pos..].find("<response>").or_else(|| xml[pos..].find("<d:response>")) {
        let block_start = pos + start;
        let end_tag = if xml[block_start..].contains("</response>") { "</response>" } else { "</d:response>" };
        let Some(block_end) = xml[block_start..].find(end_tag) else { break; };
        let block = &xml[block_start..block_start + block_end + end_tag.len()];
        pos = block_start + block_end + end_tag.len();

        let href = extract_xml_value(block, "href").unwrap_or_default();
        if href.is_empty() {
            continue;
        }
        let etag = extract_xml_value(block, "getetag")
            .map(|e| e.trim_matches('"').to_string());
        let content_type = extract_xml_value(block, "getcontenttype");

        let full_href = if href.starts_with('/') {
            format!("{}{}", base_url, href)
        } else if href.starts_with("http") {
            href
        } else {
            continue;
        };

        resources.push(DavResource { href: full_href, etag, content_type });
    }
    resources
}

fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    // Try with and without namespace prefix
    for prefix in &["", "d:"] {
        let open = format!("<{}{}>", prefix, tag);
        let close = format!("</{}{}>", prefix, tag);
        if let Some(start) = xml.find(&open) {
            if let Some(end) = xml[start + open.len()..].find(&close) {
                return Some(xml[start + open.len()..start + open.len() + end].to_string());
            }
        }
    }
    None
}
