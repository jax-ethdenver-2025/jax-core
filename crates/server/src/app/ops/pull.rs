use clap::Args;
use iroh::{discovery::pkarr::dht::DhtDiscovery, Endpoint};
use iroh_blobs::{
    net_protocol::Blobs,
    store::{ExportFormat, ExportMode},
    ticket::BlobTicket,
};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::PathBuf;

use crate::app::Op;

#[derive(Debug, Clone, Args)]
pub struct Pull {
    /// Ticket string for the blob to pull
    #[arg(short, long)]
    pub ticket: String,

    /// Path where to save the downloaded file
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum PullError {
    #[error(transparent)]
    Default(#[from] anyhow::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Invalid ticket format: {0}")]
    InvalidTicket(String),
}

#[async_trait::async_trait]
impl Op for Pull {
    type Error = PullError;
    type Output = PathBuf;

    async fn execute(&self) -> Result<Self::Output, Self::Error> {
        // connect to the mainline dht as an ephemeral node
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0); // Let system choose port
        let mainline_discovery = DhtDiscovery::builder()
            .build()
            .map_err(PullError::Default)?;
        // Create the endpoint with our key and discovery
        let endpoint = Endpoint::builder()
            .discovery(Box::new(mainline_discovery))
            .bind_addr_v4(addr)
            .bind()
            .await
            .map_err(PullError::Default)?;
        // and create a short lived blobs client to download the blob
        let blobs = Blobs::memory().build(&endpoint);
        let blobs_client = blobs.client();

        // Parse the ticket
        let ticket: BlobTicket = self
            .ticket
            .parse()
            .map_err(|_| PullError::InvalidTicket(self.ticket.clone()))?;

        // Download the blob
        blobs_client
            .download(ticket.hash(), ticket.node_addr().clone())
            .await?
            .finish()
            .await?;

        // Get absolute path for output
        let abs_path = std::path::absolute(&self.output)?;

        // Export the blob to the specified path
        blobs_client
            .export(
                ticket.hash(),
                abs_path.clone(),
                ExportFormat::Blob,
                ExportMode::Copy,
            )
            .await?
            .finish()
            .await?;

        Ok(abs_path)
    }
}
