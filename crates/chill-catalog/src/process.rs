use std::{process::ExitStatus, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Child,
};

pub async fn read_limited(reader: impl AsyncRead + Unpin, limit: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| "Lecture du processus interrompue.")?;
    if bytes.len() > limit {
        return Err("Limite de mémoire atteinte. Aucune modification du catalogue.".into());
    }
    Ok(bytes)
}

pub async fn capture(
    mut child: Child,
    limit: usize,
    timeout: Duration,
) -> Result<(ExitStatus, Vec<u8>), String> {
    // The owner configures kill_on_drop(true). Dropping on timeout/oversize stops the child.
    let stdout = child
        .stdout
        .take()
        .ok_or("Sortie du processus indisponible.")?;
    tokio::time::timeout(timeout, async move {
        let bytes = read_limited(stdout, limit).await?;
        let status = child.wait().await.map_err(|_| "Processus interrompu.")?;
        Ok((status, bytes))
    })
    .await
    .map_err(|_| "Délai dépassé. Aucune modification du catalogue.")?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn caps_output_before_collecting_large_data() {
        assert!(read_limited(&b"123456"[..], 5).await.is_err());
        assert_eq!(read_limited(&b"12345"[..], 5).await.unwrap(), b"12345");
    }
}
