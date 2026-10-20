use chill_server::{
    api::{AppState, DatabaseActor, router, router_http},
    config::{LibrarySettings, ServerConfig, prepare_data_dir},
    connection::{FirstByteAcceptor, LimitedAcceptor},
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
    if arguments.len() == 1 && arguments[0] == "--version" {
        println!("chill-server {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
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
        tmdb_jobs: Arc::new(chill_server::tmdb_api::TmdbJobService::initialize(
            &config.data_dir,
        )?),
        playback: Default::default(),
        scan: Arc::new(Mutex::new(chill_server::catalog_api::ScanStatus::default())),
        database: DatabaseActor::open(config.data_dir.join("catalog.sqlite"))?,
        library: Arc::new(Mutex::new(LibrarySettings::load(&config)?)),
        certificate: Arc::new(Mutex::new(certificates.schedule)),
        config: Arc::new(config),
    };
    // Bind before publishing a LAN hint. `axum_server::bind` defers this until
    // polling `serve`, so it cannot establish that an advertised endpoint is up.
    let https_listener =
        std::net::TcpListener::bind(state.config.listen).map_err(|_| "Écoute HTTPS impossible.")?;
    https_listener
        .set_nonblocking(true)
        .map_err(|_| "Écoute HTTPS impossible.")?;
    let acceptor = axum_server::tls_rustls::RustlsAcceptor::new(rustls.clone())
        .handshake_timeout(std::time::Duration::from_secs(5))
        .acceptor(LimitedAcceptor::new(16));
    let mut server = axum_server::from_tcp(https_listener)
        .map_err(|_| "Écoute HTTPS impossible.")?
        .acceptor(acceptor);
    tune(server.http_builder());
    // A best-effort LAN hint only: the client still validates the configured
    // HTTPS SAN and its pinned certificate before it can connect.
    let discovery_server_id = state
        .certificate
        .lock()
        .map_err(|_| "Certificat HTTPS indisponible.")?
        .server_id
        .clone();
    let _discovery_advertiser = chill_server::discovery::start(
        &discovery_server_id,
        env!("CARGO_PKG_VERSION"),
        state.config.listen,
    );
    let renewal_state = state.clone();
    chill_server::updates_api::spawn_status_pulse(state.clone());
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
            let Some(_write_guard) = chill_server::updates_api::try_background_write() else {
                continue;
            };
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
    // Optional plain-HTTP listener for the bundled web client (ADR-0018); same limits, same router, other scheme.
    let http_server = state.config.http_listen.map(|address| {
        let mut http = axum_server::bind(address).acceptor(FirstByteAcceptor::new(
            16,
            std::time::Duration::from_secs(5),
        ));
        // No protocol sniffing in clear text: browsers speak HTTP/1 here, and the sniffing step has no timeout, so 16 silent
        // sockets would starve the pool (review 6). `http1_only` goes straight to the HTTP/1 read, whose header timeout applies.
        *http.http_builder() =
            hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                .http1_only();
        http.http_builder()
            .http1()
            .timer(hyper_util::rt::TokioTimer::new())
            .header_read_timeout(std::time::Duration::from_secs(5))
            .keep_alive(false);
        http.serve(router_http(state.clone()).into_make_service())
    });
    let https_server = server.serve(router(state).into_make_service());
    match http_server {
        Some(http_server) => tokio::select! {
            result = https_server => result.map_err(|_| "Service HTTPS arrêté sur erreur.".to_string()),
            result = http_server => result.map_err(|_| "Service HTTP arrêté sur erreur.".to_string()),
        },
        None => https_server
            .await
            .map_err(|_| "Service HTTPS arrêté sur erreur.".to_string()),
    }
}

fn tune(builder: &mut hyper_util::server::conn::auto::Builder<hyper_util::rt::TokioExecutor>) {
    builder
        .http1()
        .timer(hyper_util::rt::TokioTimer::new())
        .header_read_timeout(std::time::Duration::from_secs(5))
        .keep_alive(false);
    builder
        .http2()
        .timer(hyper_util::rt::TokioTimer::new())
        .max_concurrent_streams(8)
        .keep_alive_interval(Some(std::time::Duration::from_secs(10)))
        .keep_alive_timeout(std::time::Duration::from_secs(5));
}
