/// Mailora v2 — Payload Limit Middleware
/// 
/// This is enforced at the actix-web framework level via:
/// - `web::PayloadConfig::new(10MB)` — global request body limit
/// - `web::JsonConfig::default().limit(10MB)` — JSON body limit
/// - `MultipartFormConfig::total_limit(10MB)` — multipart upload limit
///
/// When a request exceeds these limits, actix-web automatically returns:
/// HTTP 413 Payload Too Large
///
/// The request is rejected BEFORE it reaches any route handler.
/// The body is NOT buffered into memory — actix reads it as a stream
/// and aborts when the limit is exceeded.
///
/// No custom middleware code needed — actix handles this natively.
/// This file documents the design decision.
///
/// To test:
/// ```bash
/// # Generate a 15MB file
/// dd if=/dev/zero of=/tmp/big.bin bs=1M count=15
/// 
/// # Send it — expect 413
/// curl -X POST http://localhost:3030/api/send \
///   -F "file=@/tmp/big.bin" \
///   -w "\nHTTP Status: %{http_code}\n"
/// ```
///
/// Expected output: HTTP Status: 413
