use chill_server::{
    api::{AppState, DatabaseActor, router},
    config::{LibrarySettings, ServerConfig, prepare_data_dir},
    connection::LimitedAcceptor,
    tls,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Chill Center : {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() == 2 && arguments[0] == "--scan-worker" {
        let discovered = chill_catalog::scanner::discover(std::path::Path::new(&arguments[1]))?;
        return serde_json::to_writer(std::io::stdout().lock(), &discovered)
            .map_err(|_| "Scan interrompu.".into());
    }
    if arguments.len() != 2 || arguments[0] != "--config" {
        return Err("Usage : chill-server --config /chemin/config.json".into());
    }
    let config_path = PathBuf::from(&arguments[1]);
    let config = ServerConfig::load(&config_path)?;
    prepare_data_dir(&config.data_dir)?;
    // Service-owned SQLite/WAL and future staging files must not inherit a permissive shell umask.
    unsafe {
        libc::umask(0o077);
    }
    let certificates = tls::ensure(&config, time::OffsetDateTime::now_utc())?;
    let rustls = tls::load(&certificates).await?;
    let state = AppState {
        playback: Default::default(),
        scan: Arc::new(Mutex::new(chill_server::catalog_api::ScanStatus::default())),
        database: DatabaseActor::open(config.data_dir.join("catalog.sqlite"))?,
        library: Arc::new(Mutex::new(LibrarySettings::load(&config)?)),
        certificate: Arc::new(Mutex::new(certificates.schedule)),
        config: Arc::new(config),
    };
    let renewal_state = state.clone();
    if chill_server::archive_api::initialize_staging(&state).is_err(){eprintln!("IMPORT_STAGING_UNAVAILABLE");}
    let renewal_tls = rustls.clone();
    tokio::spawn(async move {
        let mut daily = tokio::time::interval(std::time::Duration::from_secs(60));
        daily.tick().await;
        loop {
            daily.tick().await;
            let config = renewal_state.config.clone();
            let renewed = tokio::task::spawn_blocking(move || {
                tls::ensure(&config, time::OffsetDateTime::now_utc())
            })
            .await;
            if let Ok(Ok(certificates)) = renewed {
                if renewal_tls
                    .reload_from_pem_file(&certificates.certificate_path, &certificates.key_path)
                    .await
                    .is_ok()
                {
                    if let Ok(mut schedule) = renewal_state.certificate.lock() {
                        *schedule = certificates.schedule;
                    }
                } else {
                    eprintln!("TLS_RELOAD_FAILED");
                }
            } else {
                eprintln!("TLS_RENEWAL_FAILED");
            }
        }
    });
    let acceptor = axum_server::tls_rustls::RustlsAcceptor::new(rustls)
        .handshake_timeout(std::time::Duration::from_secs(5))
        .acceptor(LimitedAcceptor::new(16));
    let mut server = axum_server::bind(state.config.listen).acceptor(acceptor);
    server
        .http_builder()
        .http1()
        .timer(hyper_util::rt::TokioTimer::new())
        .header_read_timeout(std::time::Duration::from_secs(5))
        .keep_alive(false);
    server
        .http_builder()
        .http2()
        .timer(hyper_util::rt::TokioTimer::new())
        .max_concurrent_streams(8)
        .keep_alive_interval(Some(std::time::Duration::from_secs(10)))
        .keep_alive_timeout(std::time::Duration::from_secs(5));
    server
        .serve(router(state).into_make_service())
        .await
        .map_err(|_| "Service HTTPS arrêté sur erreur.".to_string())
}
