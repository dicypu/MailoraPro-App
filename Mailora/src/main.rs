use actix_web::{web, App, HttpServer, middleware};
use actix_files::Files;
use actix_multipart::form::MultipartFormConfig;

mod config;
mod routes;
mod middleware as mw;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT").unwrap_or_else(|_| "3030".to_string()).parse().unwrap();
    let max_payload: usize = std::env::var("MAX_PAYLOAD_SIZE")
        .unwrap_or_else(|_| "10485760".to_string()).parse().unwrap(); // 10MB

    log::info!("Mailora v2 starting on {}:{}", host, port);
    log::info!("Max payload size: {} bytes ({} MB)", max_payload, max_payload / 1048576);

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            // === PAYLOAD LIMIT MIDDLEWARE ===
            // All requests are rejected with 413 if payload exceeds limit
            // This happens BEFORE the request reaches any route handler
            .app_data(web::PayloadConfig::new(max_payload))
            .app_data(web::JsonConfig::default().limit(max_payload))
            .app_data(
                MultipartFormConfig::default()
                    .total_limit(max_payload)
                    .memory_limit(2 * 1024 * 1024) // 2MB in memory, rest to temp file
            )
            // Routes
            .configure(routes::configure)
            // Static files (frontend)
            .service(Files::new("/static", "./static").index_file("index.html"))
            // Root redirect to /static/index.html
            .route("/", web::get().to(|| async {
                actix_web::HttpResponse::Found()
                    .append_header(("Location", "/static/index.html"))
                    .finish()
            }))
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}
