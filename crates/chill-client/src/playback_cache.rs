//! Device-local compressed packet budgets, never server/profile preferences.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaybackCache {
    pub forward_mib: u16,
    pub back_mib: u16,
}

impl Default for PlaybackCache {
    fn default() -> Self {
        Self {
            forward_mib: 128,
            back_mib: 64,
        }
    }
}

impl PlaybackCache {
    pub fn validate(self) -> Result<(), &'static str> {
        if !(16..=512).contains(&self.forward_mib) || self.back_mib > 256 {
            return Err("Tampon avant : 16 à 512 Mio ; arrière : 0 à 256 Mio.");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budgets_are_bounded_and_zero_disables_back_cache() {
        assert!(PlaybackCache::default().validate().is_ok());
        for (forward_mib, back_mib) in [(16, 0), (512, 256)] {
            assert!(
                PlaybackCache {
                    forward_mib,
                    back_mib
                }
                .validate()
                .is_ok()
            );
        }
        for (forward_mib, back_mib) in [(0, 64), (15, 64), (513, 0), (128, 257)] {
            assert!(
                PlaybackCache {
                    forward_mib,
                    back_mib
                }
                .validate()
                .is_err()
            );
        }
        for value in [
            r#"{"forward_mib":-1,"back_mib":0}"#,
            r#"{"forward_mib":16.5,"back_mib":0}"#,
            r#"{"forward_mib":128,"back_mib":64,"url":"http://invalid"}"#,
        ] {
            assert!(serde_json::from_str::<PlaybackCache>(value).is_err());
        }
    }
}
