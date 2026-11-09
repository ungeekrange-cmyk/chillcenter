//! Read only container headers, never packets, decode, transcode or spawn tools.
//! References: RFC9559 Info/Duration; QTFF mvhd; Microsoft AVIMAINHEADER.
use std::{
    fs::File,
    os::unix::fs::FileExt,
    time::{Duration, Instant},
};

struct Header {
    id: u64,
    body: u64,
    end: u64,
}
struct Reader<'a> {
    file: &'a File,
    size: u64,
    bytes: usize,
    steps: usize,
    until: Instant,
}
impl Reader<'_> {
    fn read(&mut self, offset: u64, len: usize) -> Option<Vec<u8>> {
        self.steps += 1;
        self.bytes = self.bytes.checked_add(len)?;
        if self.steps > 2048
            || self.bytes > 64 * 1024
            || Instant::now() > self.until
            || offset.checked_add(len as u64)? > self.size
        {
            return None;
        }
        let mut bytes = vec![0; len];
        self.file.read_exact_at(&mut bytes, offset).ok()?;
        Some(bytes)
    }
    fn atom(&mut self, pos: u64, end: u64) -> Option<Header> {
        if pos.checked_add(8)? > end {
            return None;
        }
        let bytes = self.read(pos, 8)?;
        let n = u32::from_be_bytes(bytes[..4].try_into().ok()?) as u64;
        let (size, header) = match n {
            0 => (end.checked_sub(pos)?, 8),
            1 => (
                u64::from_be_bytes(self.read(pos + 8, 8)?.try_into().ok()?),
                16,
            ),
            _ => (n, 8),
        };
        let next = pos.checked_add(size)?;
        if size < header || next > end {
            return None;
        }
        Some(Header {
            id: u32::from_be_bytes(bytes[4..8].try_into().ok()?) as u64,
            body: pos + header,
            end: next,
        })
    }
    fn movie(&mut self) -> Option<u64> {
        let mut pos = 0;
        while let Some(atom) = self.atom(pos, self.size) {
            if atom.id == u32::from_be_bytes(*b"moov") as u64 {
                let mut child = atom.body;
                while let Some(h) = self.atom(child, atom.end) {
                    if h.id == u32::from_be_bytes(*b"mvhd") as u64 {
                        let version = *self.read(h.body, 1)?.first()?;
                        let (scale_at, duration_at, len) = match version {
                            0 => (12, 16, 4),
                            1 => (20, 24, 8),
                            _ => return None,
                        };
                        if h.body.checked_add(duration_at + len)? > h.end {
                            return None;
                        }
                        let scale =
                            u32::from_be_bytes(self.read(h.body + scale_at, 4)?.try_into().ok()?);
                        let raw = self.read(h.body + duration_at, len as usize)?;
                        let duration = if len == 4 {
                            u32::from_be_bytes(raw.try_into().ok()?) as u64
                        } else {
                            u64::from_be_bytes(raw.try_into().ok()?)
                        };
                        if scale == 0
                            || duration == u64::MAX
                            || (len == 4 && duration == u32::MAX as u64)
                        {
                            return None;
                        }
                        return u64::try_from(
                            (duration as u128).checked_mul(1000)? / scale as u128,
                        )
                        .ok()
                        .and_then(valid);
                    }
                    child = h.end;
                }
                return None;
            }
            pos = atom.end;
        }
        None
    }
    fn ebml(&mut self, pos: u64, end: u64, unknown_ok: bool) -> Option<Header> {
        let first = *self.read(pos, 1)?.first()?;
        let id_len = first.leading_zeros() as usize + 1;
        if id_len > 4 {
            return None;
        }
        let id = uint(&self.read(pos, id_len)?);
        let p = pos.checked_add(id_len as u64)?;
        let first = *self.read(p, 1)?.first()?;
        let len = first.leading_zeros() as usize + 1;
        if len > 8 {
            return None;
        }
        let mut bytes = self.read(p, len)?;
        bytes[0] &= if len == 8 { 0 } else { 0xffu8 >> len };
        let size = uint(&bytes);
        let unknown = size == (1u64 << (len * 7)) - 1;
        let body = p.checked_add(len as u64)?;
        let next = if unknown && unknown_ok {
            end
        } else if unknown {
            return None;
        } else {
            body.checked_add(size)?
        };
        if next > end || body > next {
            return None;
        }
        Some(Header {
            id,
            body,
            end: next,
        })
    }
    fn matroska(&mut self) -> Option<u64> {
        let mut pos = 0;
        while let Some(h) = self.ebml(pos, self.size, true) {
            if h.id == 0x18538067 {
                let mut p = h.body;
                while p < h.end {
                    let info = self.ebml(p, h.end, false)?;
                    if info.id == 0x1549a966 {
                        let mut p = info.body;
                        let mut scale = 1_000_000;
                        let mut duration = None;
                        while p < info.end {
                            let item = self.ebml(p, info.end, false)?;
                            if item.id == 0x2ad7b1 {
                                let len = item.end.checked_sub(item.body)?;
                                if !(1..=8).contains(&len) {
                                    return None;
                                }
                                scale = uint(&self.read(item.body, len as usize)?);
                            } else if item.id == 0x4489 {
                                let len = item.end.checked_sub(item.body)?;
                                duration = Some(match len {
                                    4 => f32::from_be_bytes(
                                        self.read(item.body, 4)?.try_into().ok()?,
                                    ) as f64,
                                    8 => f64::from_be_bytes(
                                        self.read(item.body, 8)?.try_into().ok()?,
                                    ),
                                    _ => return None,
                                });
                            }
                            p = item.end;
                        }
                        let ms = duration? * scale as f64 / 1_000_000.0;
                        return (ms.is_finite() && ms > 0.0 && ms < 9_007_199_254_740_991.0)
                            .then(|| ms.round() as u64)
                            .and_then(valid);
                    }
                    // Unknown-size clusters are refused; do not walk packet data.
                    p = info.end;
                }
            }
            pos = h.end;
        }
        None
    }
    fn avi_chunks(
        &mut self,
        mut pos: u64,
        end: u64,
        depth: u8,
        micros: &mut u64,
        frames: &mut u64,
    ) -> Option<()> {
        if depth > 4 {
            return None;
        }
        while pos.checked_add(8)? <= end {
            let h = self.read(pos, 8)?;
            let size = u32::from_le_bytes(h[4..8].try_into().ok()?) as u64;
            let body = pos.checked_add(8)?;
            let next = body.checked_add(size)?;
            if next > end {
                return None;
            }
            match &h[..4] {
                b"LIST" if size >= 4 => {
                    let kind = self.read(body, 4)?;
                    if kind == b"hdrl" || kind == b"odml" {
                        self.avi_chunks(body + 4, next, depth + 1, micros, frames)?;
                    }
                }
                b"avih" if size >= 20 => {
                    let v = self.read(body, 20)?;
                    *micros = u32::from_le_bytes(v[..4].try_into().ok()?) as u64;
                    *frames = (*frames).max(u32::from_le_bytes(v[16..20].try_into().ok()?) as u64);
                }
                b"dmlh" if size >= 4 => {
                    *frames = (*frames)
                        .max(u32::from_le_bytes(self.read(body, 4)?.try_into().ok()?) as u64)
                }
                _ => (),
            }
            pos = next.checked_add(size & 1)?;
        }
        Some(())
    }
    fn avi(&mut self) -> Option<u64> {
        let h = self.read(0, 12)?;
        if &h[..4] != b"RIFF" || &h[8..12] != b"AVI " {
            return None;
        }
        let end = (u32::from_le_bytes(h[4..8].try_into().ok()?) as u64).checked_add(8)?;
        if end > self.size {
            return None;
        }
        let (mut micros, mut frames) = (0, 0);
        self.avi_chunks(12, end, 0, &mut micros, &mut frames)?;
        micros.checked_mul(frames).map(|v| v / 1000).and_then(valid)
    }
}
fn uint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0, |n, b| (n << 8) | *b as u64)
}
fn valid(ms: u64) -> Option<u64> {
    (ms > 0 && ms <= 9_007_199_254_740_991).then_some(ms)
}
pub(crate) fn duration_ms(file: &File) -> Option<u64> {
    let mut r = Reader {
        file,
        size: file.metadata().ok()?.len(),
        bytes: 0,
        steps: 0,
        until: Instant::now() + Duration::from_millis(200),
    };
    let h = r.read(0, 12)?;
    if &h[..4] == b"\x1a\x45\xdf\xa3" {
        r.matroska()
    } else if &h[..4] == b"RIFF" {
        r.avi()
    } else {
        r.movie()
    }
}

pub(crate) fn probe_many(
    candidates: Vec<(String, std::path::PathBuf, std::path::PathBuf)>,
) -> std::collections::HashMap<String, u64> {
    use crate::media_stream::{FileVersion, open_indexed};
    use std::{
        collections::HashMap,
        sync::{Mutex, OnceLock},
    };
    type Cache = HashMap<(std::path::PathBuf, FileVersion), Option<u64>>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let until = Instant::now() + Duration::from_secs(2);
    let mut found = HashMap::new();
    for (id, root, path) in candidates.into_iter().take(20) {
        if Instant::now() > until {
            break;
        }
        let Ok((file, version)) = open_indexed(&root, &path) else {
            continue;
        };
        let key = (path, version.clone());
        let cached = cache.lock().ok().and_then(|c| c.get(&key).copied());
        let duration = match cached {
            Some(value) => value,
            None => {
                let value = duration_ms(&file);
                if file
                    .metadata()
                    .ok()
                    .and_then(|m| FileVersion::from_metadata(&m).ok())
                    .as_ref()
                    != Some(&version)
                {
                    continue;
                }
                if let Ok(mut c) = cache.lock() {
                    if c.len() >= 512 {
                        c.clear();
                    }
                    c.insert(key, value);
                }
                value
            }
        };
        if let Some(value) = duration {
            found.insert(id, value);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn probe(bytes: &[u8]) -> Option<u64> {
        let mut f = tempfile::tempfile().unwrap();
        f.write_all(bytes).unwrap();
        duration_ms(&f)
    }
    fn atom(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        [(body.len() as u32 + 8).to_be_bytes().as_slice(), kind, body].concat()
    }
    #[test]
    fn movie_versions_and_invalid_timescales() {
        for version in [0, 1] {
            let mut b = vec![0; if version == 0 { 20 } else { 32 }];
            b[0] = version;
            let p = if version == 0 { 12 } else { 20 };
            b[p..p + 4].copy_from_slice(&1000u32.to_be_bytes());
            if version == 0 {
                b[16..20].copy_from_slice(&123456u32.to_be_bytes());
            } else {
                b[24..32].copy_from_slice(&123456u64.to_be_bytes());
            }
            assert_eq!(probe(&atom(b"moov", &atom(b"mvhd", &b))), Some(123456));
            b[p..p + 4].fill(0);
            assert_eq!(probe(&atom(b"moov", &atom(b"mvhd", &b))), None);
        }
    }

    #[test]
    fn replacing_a_file_invalidates_cached_duration_and_symlinks_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let path = root.join("video.mp4");
        let movie = |ms: u32| {
            let mut h = vec![0; 20];
            h[12..16].copy_from_slice(&1000u32.to_be_bytes());
            h[16..20].copy_from_slice(&ms.to_be_bytes());
            atom(b"moov", &atom(b"mvhd", &h))
        };
        std::fs::write(&path, movie(10000)).unwrap();
        let read = || probe_many(vec![("id".into(), root.clone(), path.clone())]);
        assert_eq!(read().get("id"), Some(&10000));
        let replacement = root.join("replacement.mp4");
        std::fs::write(&replacement, movie(20000)).unwrap();
        std::fs::rename(&replacement, &path).unwrap();
        assert_eq!(read().get("id"), Some(&20000));
        let link = root.join("alias.mp4");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(probe_many(vec![("id".into(), root, link)]).is_empty());
    }
    #[test]
    fn matroska_float_default_and_explicit_scale() {
        let ebml = [0x1a, 0x45, 0xdf, 0xa3, 0x80];
        for explicit in [false, true] {
            let mut info = vec![];
            if explicit {
                info.extend([0x2a, 0xd7, 0xb1, 0x83, 0x0f, 0x42, 0x40]);
            }
            info.extend([0x44, 0x89, 0x88]);
            info.extend(123456f64.to_be_bytes());
            let data = [
                ebml.as_slice(),
                &[
                    0x18,
                    0x53,
                    0x80,
                    0x67,
                    0xff,
                    0x15,
                    0x49,
                    0xa9,
                    0x66,
                    0x80 | info.len() as u8,
                ],
                &info,
            ]
            .concat();
            assert_eq!(probe(&data), Some(123456));
        }
    }
    #[test]
    fn avi_declared_frames_and_malformed_inputs() {
        let mut main = vec![0; 56];
        main[..4].copy_from_slice(&40000u32.to_le_bytes());
        main[16..20].copy_from_slice(&250u32.to_le_bytes());
        let chunk = [b"avih".as_slice(), &56u32.to_le_bytes(), &main].concat();
        let list = [
            b"LIST".as_slice(),
            &(chunk.len() as u32 + 4).to_le_bytes(),
            b"hdrl",
            &chunk,
        ]
        .concat();
        let file = [
            b"RIFF".as_slice(),
            &(list.len() as u32 + 4).to_le_bytes(),
            b"AVI ",
            &list,
        ]
        .concat();
        assert_eq!(probe(&file), Some(10000));
        for n in 0..file.len() {
            assert_eq!(probe(&file[..n]), None);
        }
        assert_eq!(probe(&[0; 4096]), None);
    }
}
