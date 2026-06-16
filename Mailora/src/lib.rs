// filepath: /mailora-hub-imap/mailora-hub-imap/src/lib.rs
pub mod config;
pub mod db;
pub mod imap;
pub mod models;
pub mod oauth;
pub mod persist;
pub mod pim;
pub mod rbac;
pub mod routes;
pub mod services;
pub mod smtp;
#[path = "telemetry/mod.rs"]
pub mod telemetry; // explicitly use directory module
                   // pub mod stalwart_client; // Deprecated - using direct IMAP/SMTP now

// static dizini altındaki kaynakları include_str! ile alıyoruz.
// const _: () = {
//     #[cfg_attr(not(feature = "embed-static"), allow(unused_variables))]
//     let _ = {
//         use std::path::PathBuf;
//         let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
//         path.push("static");
//         let _ = std::fs::create_dir_all(&path);
//         let _ = std::fs::copy("static/index.html", path.join("index.html"));
//         let _ = std::fs::copy("static/style.css", path.join("style.css"));
//         let _ = std::fs::copy("static/script.js", path.join("script.js"));
//     };
// };
