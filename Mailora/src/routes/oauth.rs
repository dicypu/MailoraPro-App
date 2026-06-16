use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::oauth::OAuthManager;

#[derive(Deserialize)]
pub struct StartAuthQuery {
    provider: String,
    account_id: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

#[derive(Serialize)]
pub struct AuthUrlResponse {
    auth_url: String,
    account_id: String,
}

/// Start OAuth flow - returns authorization URL
pub async fn start_oauth(
    State(oauth_manager): State<Arc<OAuthManager>>,
    Query(params): Query<StartAuthQuery>,
) -> Result<Json<AuthUrlResponse>, String> {
    // Generate temporary account_id if not provided
    let account_id = params
        .account_id
        .unwrap_or_else(|| format!("temp_{}", uuid::Uuid::new_v4().to_string()));

    let auth_url = oauth_manager
        .start_auth_flow(&params.provider, &account_id)
        .await
        .map_err(|e| format!("Failed to start OAuth flow: {}", e))?;

    Ok(Json(AuthUrlResponse {
        auth_url,
        account_id,
    }))
}

/// OAuth callback - exchange code for tokens and save to DB
pub async fn oauth_callback(
    State(oauth_manager): State<Arc<OAuthManager>>,
    Query(params): Query<CallbackQuery>,
) -> impl IntoResponse {
    match oauth_manager
        .handle_callback(params.code, params.state)
        .await
    {
        Ok((provider, _account_id, tokens)) => {
            // Success - show page with postMessage to parent window
            let html = format!(
                r#"
<!DOCTYPE html>
<html>
<head>
    <title>OAuth Success</title>
    <style>
        body {{
            font-family: system-ui;
            background: #0f1217;
            color: #e5e7eb;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
        }}
        .container {{
            background: #1b2230;
            border: 1px solid #243043;
            border-radius: 12px;
            padding: 40px;
            max-width: 600px;
            text-align: center;
        }}
        .success {{
            font-size: 48px;
            margin-bottom: 20px;
        }}
        h1 {{
            color: #10b981;
            margin-bottom: 10px;
        }}
        .info {{
            background: #111827;
            border: 1px solid #1f2937;
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
            text-align: left;
        }}
        .info p {{
            margin: 8px 0;
            font-size: 14px;
        }}
        button {{
            background: #1e40af;
            color: white;
            border: none;
            padding: 12px 24px;
            border-radius: 8px;
            cursor: pointer;
            font-size: 14px;
            font-weight: 500;
            margin-top: 20px;
        }}
        button:hover {{
            background: #1e3a8a;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="success">✅</div>
        <h1>Authentication Successful!</h1>
        <p>Your {} account has been authorized.</p>
        
        <div class="info">
            <p><strong>Provider:</strong> {}</p>
            <p><strong>Has Refresh Token:</strong> {}</p>
            <p style="margin-top: 12px; color: #10b981;">
                ✓ Account will be automatically added to your mailbox
            </p>
        </div>
        
        <button onclick="window.close()">Close Window</button>
        
        <script>
            // Send tokens back to parent window
            if (window.opener) {{
                window.opener.postMessage({{
                    type: 'oauth-success',
                    provider: '{}',
                    tokens: {{
                        accessToken: '{}',
                        refreshToken: '{}',
                        expiresAt: {}
                    }}
                }}, '*');
                
                // Auto-close after 2 seconds
                setTimeout(() => window.close(), 2000);
            }}
        </script>
    </div>
</body>
</html>
                "#,
                provider,
                provider,
                tokens.refresh_token.is_some(),
                provider,
                tokens.access_token,
                tokens.refresh_token.unwrap_or_default(),
                tokens.expires_at.unwrap_or(0)
            );

            Html(html)
        }
        Err(e) => {
            let html = format!(
                r#"
<!DOCTYPE html>
<html>
<head>
    <title>OAuth Error</title>
    <style>
        body {{
            font-family: system-ui;
            background: #0f1217;
            color: #e5e7eb;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
        }}
        .container {{
            background: #1b2230;
            border: 1px solid #243043;
            border-radius: 12px;
            padding: 40px;
            max-width: 600px;
            text-align: center;
        }}
        .error {{
            font-size: 48px;
            margin-bottom: 20px;
        }}
        h1 {{
            color: #ef4444;
            margin-bottom: 10px;
        }}
        .error-msg {{
            background: #111827;
            border: 1px solid #991b1b;
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
            color: #fca5a5;
        }}
        button {{
            background: #374151;
            color: white;
            border: none;
            padding: 12px 24px;
            border-radius: 8px;
            cursor: pointer;
            font-size: 14px;
            font-weight: 500;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="error">❌</div>
        <h1>Authentication Failed</h1>
        <div class="error-msg">{}</div>
        <button onclick="window.close()">Close Window</button>
    </div>
</body>
</html>
                "#,
                e
            );

            Html(html)
        }
    }
}

#[derive(Serialize)]
pub struct OAuthSetupGuide {
    provider: String,
    setup_url: String,
    instructions: Vec<String>,
}

/// Get OAuth setup instructions for providers
pub async fn oauth_setup_guide(Query(params): Query<StartAuthQuery>) -> Json<OAuthSetupGuide> {
    let (setup_url, instructions) = match params.provider.as_str() {
        "gmail" => (
            "https://console.cloud.google.com/apis/credentials",
            vec![
                "Go to Google Cloud Console".to_string(),
                "Create new OAuth 2.0 Client ID".to_string(),
                "Application type: Web application".to_string(),
                "Authorized redirect URIs: http://localhost:3030/oauth/callback".to_string(),
                "Enable Gmail API".to_string(),
                "Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET env vars".to_string(),
            ],
        ),
        "outlook" => (
            "https://portal.azure.com/#view/Microsoft_AAD_RegisteredApps/ApplicationsListBlade",
            vec![
                "Go to Azure Portal > App registrations".to_string(),
                "New registration".to_string(),
                "Redirect URI: http://localhost:3030/oauth/callback".to_string(),
                "API permissions: Add Microsoft Graph Mail.ReadWrite, offline_access".to_string(),
                "Certificates & secrets: New client secret".to_string(),
                "Set MICROSOFT_CLIENT_ID and MICROSOFT_CLIENT_SECRET env vars".to_string(),
            ],
        ),
        "yahoo" => (
            "https://developer.yahoo.com/apps/create/",
            vec![
                "Go to Yahoo Developer Portal".to_string(),
                "Create App".to_string(),
                "Select Mail API".to_string(),
                "Redirect URI: http://localhost:3030/oauth/callback".to_string(),
                "Set YAHOO_CLIENT_ID and YAHOO_CLIENT_SECRET env vars".to_string(),
            ],
        ),
        _ => (
            "",
            vec!["OAuth not supported for this provider".to_string()],
        ),
    };

    Json(OAuthSetupGuide {
        provider: params.provider,
        setup_url: setup_url.to_string(),
        instructions,
    })
}
