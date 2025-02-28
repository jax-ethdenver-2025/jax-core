use askama::Template;
use askama_axum::IntoResponse;
use iroh::NodeId;
use iroh_blobs::Hash;
use std::time::Duration;

#[derive(Template)]
#[template(path = "share.html")]
struct ShareTemplate {
    message: Option<String>,
}

// #[derive(Template)]
// #[template(path = "probe.html")]
// struct ProbeTemplate {
//     message: Option<String>,
//     stats: Option<ProbeStats>,
// }

#[derive(Template)]
#[template(path = "query.html")]
struct QueryTemplate {
    hash: Option<Hash>,
    nodes: Option<Vec<(NodeId, f64)>>,
    message: Option<String>,
}

struct ProbeStats {
    elapsed: Duration,
    bytes_read: u64,
    bytes_written: u64,
}

impl std::fmt::Display for ProbeStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Time: {:?}, Read: {} bytes, Written: {} bytes",
            self.elapsed, self.bytes_read, self.bytes_written
        )
    }
}

#[axum::debug_handler]
pub async fn share_form_handler() -> impl IntoResponse {
    ShareTemplate { message: None }
}

// #[axum::debug_handler]
// pub async fn probe_form_handler() -> impl IntoResponse {
//     ProbeTemplate {
//         message: None,
//         stats: None,
//     }
// }

#[axum::debug_handler]
pub async fn query_form_handler() -> impl IntoResponse {
    QueryTemplate {
        hash: None,
        nodes: None,
        message: None,
    }
}
