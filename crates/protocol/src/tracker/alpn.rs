//! Custom ALPN identifiers for protocol services

/// ALPN identifier for content tracking protocol
pub const TRACKER_ALPN: &[u8] = b"jax-tracker/1";

/// ALPN identifier for content announcement protocol
pub const ANNOUNCE_ALPN: &[u8] = b"jax-announce/1";

/// ALPN identifier for content discovery protocol
pub const DISCOVERY_ALPN: &[u8] = b"jax-discovery/1";

/// ALPN identifier for content probing protocol
pub const PROBE_ALPN: &[u8] = b"jax-probe/1";

/// Get all supported ALPN identifiers
pub fn all_alpns() -> Vec<&'static [u8]> {
    vec![
        TRACKER_ALPN,
        ANNOUNCE_ALPN,
        DISCOVERY_ALPN,
        PROBE_ALPN,
    ]
} 