use serde::{Deserialize, Serialize};
use trust_dns_resolver::TokioAsyncResolver;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub source: String, // "ispdb", "dns", "heuristic"
    pub imap: Option<ServerEndpoint>,
    pub smtp: Option<ServerEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEndpoint {
    pub host: String,
    pub port: u16,
    pub socket_type: String, // "SSL", "STARTTLS", "PLAIN"
}

// XML structures for Mozilla ISPDB
#[derive(Debug, Deserialize)]
struct ClientConfig {
    emailProvider: EmailProvider,
}

#[derive(Debug, Deserialize)]
struct EmailProvider {
    id: String,
    domain: Vec<String>,
    displayName: String,
    displayShortName: String,
    incomingServer: Vec<IncomingServer>,
    outgoingServer: Vec<OutgoingServer>,
}

#[derive(Debug, Deserialize)]
struct IncomingServer {
    #[serde(rename = "type")]
    server_type: String,
    hostname: String,
    port: u16,
    socketType: String,
    username: String,
    authentication: String,
}

#[derive(Debug, Deserialize)]
struct OutgoingServer {
    #[serde(rename = "type")]
    server_type: String,
    hostname: String,
    port: u16,
    socketType: String,
    username: String,
    authentication: String,
}

pub struct DiscoveryService {
    http_client: Client,
    resolver: Arc<TokioAsyncResolver>,
}

impl DiscoveryService {
    pub fn new() -> Self {
        let resolver = TokioAsyncResolver::tokio_from_system_conf().unwrap_or_else(|_| {
             // Fallback to google DNS if system config fails
             let config = trust_dns_resolver::config::ResolverConfig::google();
             let opts = trust_dns_resolver::config::ResolverOpts::default();
             TokioAsyncResolver::tokio(config, opts)
        });

        Self {
            http_client: Client::new(),
            resolver: Arc::new(resolver),
        }
    }

    pub async fn discover(&self, email: &str) -> Result<DiscoveryResult> {
        let domain = email.split('@').nth(1).ok_or_else(|| anyhow!("Invalid email"))?;

        // 1. Mozilla ISPDB
        if let Ok(result) = self.lookup_ispdb(domain).await {
            return Ok(result);
        }

        // 2. DNS SRV
        if let Ok(result) = self.lookup_dns_srv(domain).await {
             return Ok(result);
        }

        // 3. Heuristic
        self.lookup_heuristic(domain).await
    }

    async fn lookup_ispdb(&self, domain: &str) -> Result<DiscoveryResult> {
        let url = format!("https://autoconfig.thunderbird.net/v1.1/{}", domain);
        let resp = self.http_client.get(&url).send().await?;
        
        if !resp.status().is_success() {
            return Err(anyhow!("ISPDB lookup failed"));
        }

        let xml_content = resp.text().await?;
        let config: ClientConfig = quick_xml::de::from_str(&xml_content)?;

        // Extract best IMAP/SMTP
        let imap = config.emailProvider.incomingServer.iter()
            .find(|s| s.server_type == "imap");
            
        let smtp = config.emailProvider.outgoingServer.iter()
            .find(|s| s.server_type == "smtp");

        if let (Some(imap), Some(smtp)) = (imap, smtp) {
            Ok(DiscoveryResult {
                source: "ispdb".to_string(),
                imap: Some(ServerEndpoint {
                    host: imap.hostname.clone(),
                    port: imap.port,
                    socket_type: imap.socketType.clone(),
                }),
                smtp: Some(ServerEndpoint {
                    host: smtp.hostname.clone(),
                    port: smtp.port,
                    socket_type: smtp.socketType.clone(),
                }),
            })
        } else {
             Err(anyhow!("Incomplete config from ISPDB"))
        }
    }

    async fn lookup_dns_srv(&self, domain: &str) -> Result<DiscoveryResult> {
        // _imap._tcp.example.com
        // _submission._tcp.example.com
        
        let imap_srv = self.resolver.srv_lookup(format!("_imap._tcp.{}", domain)).await;
        let submission_srv = self.resolver.srv_lookup(format!("_submission._tcp.{}", domain)).await;

        let imap_endpoint = if let Ok(records) = imap_srv {
            // Take the first one for now (could be sorted by priority)
            records.iter().next().map(|r| ServerEndpoint {
                host: r.target().to_string().trim_end_matches('.').to_string(),
                port: r.port(),
                socket_type: "STARTTLS".to_string(), // SRV doesn't specify SSL, assume secure upgrade
            })
        } else { None };

        let smtp_endpoint = if let Ok(records) = submission_srv {
            records.iter().next().map(|r| ServerEndpoint {
                host: r.target().to_string().trim_end_matches('.').to_string(),
                port: r.port(),
                socket_type: "STARTTLS".to_string(),
            })
        } else { None };

        if imap_endpoint.is_some() || smtp_endpoint.is_some() {
             Ok(DiscoveryResult {
                source: "dns".to_string(),
                imap: imap_endpoint,
                smtp: smtp_endpoint,
            })
        } else {
            Err(anyhow!("DNS lookup failed"))
        }
    }

    async fn lookup_heuristic(&self, domain: &str) -> Result<DiscoveryResult> {
        // Guess commonly used subdomains
        // imap.domain.com, mail.domain.com
        // smtp.domain.com, mail.domain.com
        
        let imap_guesses = vec![
            format!("imap.{}", domain),
            format!("mail.{}", domain),
        ];

        let smtp_guesses = vec![
            format!("smtp.{}", domain),
            format!("mail.{}", domain),
        ];

        let mut found_imap = None;
        for host in imap_guesses {
            // Check if resolves
            if self.resolver.lookup_ip(&host).await.is_ok() {
                found_imap = Some(ServerEndpoint {
                    host,
                    port: 993, // Assume SSL
                    socket_type: "SSL".to_string(), 
                });
                break;
            }
        }
        
        let mut found_smtp = None;
        for host in smtp_guesses {
            if self.resolver.lookup_ip(&host).await.is_ok() {
                found_smtp = Some(ServerEndpoint {
                    host,
                    port: 465, // Assume SSL
                    socket_type: "SSL".to_string(),
                });
                break;
            }
        }

        if found_imap.is_some() || found_smtp.is_some() {
            Ok(DiscoveryResult {
                source: "heuristic".to_string(),
                imap: found_imap,
                smtp: found_smtp,
            })
        } else {
            Err(anyhow!("Discovery failed"))
        }
    }
}
