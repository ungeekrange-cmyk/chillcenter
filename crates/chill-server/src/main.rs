use chill_server::{
    api::{AppState, DatabaseActor, router},
    config::{LibrarySettings, ServerConfig, prepare_data_dir},
    connection::LimitedAcceptor,
    operations::{self, DataLock},
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
    if let [
        export,
        source_flag,
        source,
        cache_flag,
        cache,
        output_flag,
        output,
    ] = arguments.as_slice()
        && export == "--export-cinefolio"
        && source_flag == "--source-db"
        && cache_flag == "--image-cache"
        && output_flag == "--output"
    {
        return chill_server::legacy_export::export(
            std::path::Path::new(source),
            std::path::Path::new(cache),
            std::path::Path::new(output),
        );
    }
    if arguments.len() == 2 && arguments[0] == "--scan-worker" {
        let discovered = chill_catalog::scanner::discover(std::path::Path::new(&arguments[1]))?;
        return serde_json::to_writer(std::io::stdout().lock(), &discovered)
            .map_err(|_| "Scan interrompu.".into());
    }
    let usage = "Usage : chill-server --config /chemin/config.json\n       chill-server --backup --config /chemin/config.json\n       chill-server --restore /chemin/sauvegarde --config /chemin/config.json --confirm-replace\n       chill-server --recover --config /chemin/config.json --confirm-replace\n       chill-server --export-cinefolio --source-db /chemin/cinefolio.sqlite --image-cache /chemin/images --output /chemin/nouveau.zip";
    let (command, config_path) = match arguments.as_slice() {
        [flag, path] if flag == "--config" => ("serve", PathBuf::from(path)),
        [backup, config, path] if backup == "--backup" && config == "--config" => {
            ("backup", PathBuf::from(path))
        }
        [restore, backup, config, path, confirm]
            if restore == "--restore" && config == "--config" && confirm == "--confirm-replace" =>
        {
            let config_path = PathBuf::from(path);
            let loaded = ServerConfig::load(&config_path)?;
            let outcome =
                operations::restore(std::path::Path::new(backup), &loaded, &config_path, true)?;
            println!(
                "Restauration terminée. Anciennes données conservées : {}",
                outcome.previous_data.display()
            );
            return Ok(());
        }
        [recover, config, path, confirm]
            if recover == "--recover" && config == "--config" && confirm == "--confirm-replace" =>
        {
            let loaded = ServerConfig::load(std::path::Path::new(path))?;
            let outcome = operations::recover(&loaded, true)?;
            println!(
                "Récupération terminée : {}",
                outcome.restored_data.display()
            );
            return Ok(());
        }
        _ => return Err(usage.into()),
    };
    let config = ServerConfig::load(&config_path)?;
    if command == "backup" {
        let path = operations::backup(&config, &config_path)?;
        println!("Sauvegarde terminée : {}", path.display());
        return Ok(());
    }
    operations::ensure_no_pending(&config)?;
    prepare_data_dir(&config.data_dir)?;
    // Service-owned SQLite/WAL and future staging files must not inherit a permissive shell umask.
    unsafe {
        libc::umask(0o077);
    }
    // Held for the entire HTTPS process lifetime and acquired before TLS/SQLite.
    let _data_lock = DataLock::acquire(&config.data_dir)?;
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
    if chill_server::archive_api::initialize_staging(&state).is_err() {
        eprintln!("IMPORT_STAGING_UNAVAILABLE");
    }
    if chill_server::export_jobs::initialize(&state).is_err() {
        eprintln!("EXPORT_STAGING_UNAVAILABLE");
    }
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
