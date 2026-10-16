use std::{env, path::Path};

fn main() {
    let Some(root) = env::args().nth(1) else {
        eprintln!("usage: optimize-artwork-cache <closed-client-config-root>");
        std::process::exit(2);
    };
    match chill_client::artwork_cache::optimize_closed_cache(Path::new(&root)) {
        Ok(r) => println!(
            "ARTWORK_CACHE_OPTIMIZED logical_bytes={} before_unique_allocated_bytes={} after_unique_allocated_bytes={} unique_blob_bytes={} references={} hash_verified={}",
            r.logical_bytes,
            r.before_unique_allocated_bytes,
            r.after_unique_allocated_bytes,
            r.unique_bytes,
            r.references,
            r.verified
        ),
        Err(e) => {
            eprintln!("ARTWORK_CACHE_OPTIMIZE_FAILED {e}");
            std::process::exit(1);
        }
    }
}
