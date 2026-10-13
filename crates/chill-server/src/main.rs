use chill_server::{
    api::{AppState, DatabaseActor, router},
    config::{LibrarySettings, ServerConfig},
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
    if arguments.len() != 2 || arguments[0] != "--config" {
        return Err("Usage : chill-server --config /chemin/config.json".into());
    }
    let config_path = PathBuf::from(&arguments[1]);
    let config = ServerConfig::load(&config_path)?;
    std::fs::create_dir_all(&config.data_dir)
        .map_err(|_| "Stockage local du serveur inaccessible.")?;
    let certificates = tls::ensure(&config, time::OffsetDateTime::now_utc())?;
    let rustls = tls::load(&certificates).await?;
    let state = AppState {
        database: DatabaseActor::open(config.data_dir.join("catalog.sqlite"))?,
        library: Arc::new(Mutex::new(LibrarySettings::load(&config)?)),
        certificate: Arc::new(Mutex::new(certificates.schedule)),
        config: Arc::new(config),
    };
    let renewal_state = state.clone();
    let renewal_tls = rustls.clone();
    tokio::spawn(async move {
        let mut daily = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));
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
    axum_server::bind_rustls(state.config.listen, rustls)
        .serve(router(state).into_make_service())
        .await
        .map_err(|_| "Service HTTPS arrêté sur erreur.".to_string())
}
