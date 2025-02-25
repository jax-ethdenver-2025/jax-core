use std::sync::Arc;

use bytes::Bytes;
use clap::Args;
use iroh::{discovery::pkarr::dht::DhtDiscovery, Endpoint, NodeId};
use iroh_blobs::{
    get::{fsm::{BlobContentNext, EndBlobNext}, Stats},
    hashseq::HashSeq,
    protocol::GetRequest,
    ticket::BlobTicket,
    BlobFormat, Hash, HashAndFormat,
    protocol::RangeSpecSeq,
};
use bao_tree::{ChunkNum, ChunkRanges};
use rand::{Rng, SeedableRng};
use std::fmt;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Instant;

use crate::app::Op;

#[derive(Debug, Clone, Args)]
pub struct Probe {
    /// Ticket string for the blob to probe
    #[arg(short, long)]
    pub ticket: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error(transparent)]
    Default(#[from] anyhow::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Invalid ticket format: {0}")]
    InvalidTicket(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Probe failed: {0}")]
    ProbeFailed(String),
}

#[derive(Debug)]
pub struct ProbeResult {
    node_id: NodeId,
    content: HashAndFormat,
    stats: Stats,
    elapsed: std::time::Duration,
}

impl fmt::Display for ProbeResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Probe Result:")?;
        writeln!(f, "  Node ID: {}", self.node_id)?;
        writeln!(f, "  Content Hash: {}", self.content.hash)?;
        writeln!(f, "  Format: {:?}", self.content.format)?;
        writeln!(f, "  Time: {:.6}s", self.elapsed.as_secs_f64())?;
        writeln!(f, "  Stats:")?;
        writeln!(f, "    Bytes Read: {}", self.stats.bytes_read)?;
        writeln!(f, "    Duration: {:.6}s", self.stats.elapsed.as_secs_f64())?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Op for Probe {
    type Error = ProbeError;
    type Output = ProbeResult;

    async fn execute(&self) -> Result<Self::Output, Self::Error> {
        // Parse the ticket
        let ticket: BlobTicket = self
            .ticket
            .parse()
            .map_err(|_| ProbeError::InvalidTicket(self.ticket.clone()))?;
        
        // Extract the node ID from the node address
        let node_id = ticket.node_addr().node_id;
        let hash = ticket.hash();
        let format = BlobFormat::Raw; // Assuming raw format for simplicity
        
        let content = HashAndFormat { 
            hash, 
            format,
        };

        // Connect to the mainline DHT as an ephemeral node
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0); // Let system choose port
        let mainline_discovery = DhtDiscovery::builder()
            .build()
            .map_err(ProbeError::Default)?;
        
        // Create the endpoint with our key and discovery
        let endpoint = Endpoint::builder()
            .discovery(Box::new(mainline_discovery))
            .bind_addr_v4(addr)
            .bind()
            .await
            .map_err(ProbeError::Default)?;

        // Connect to the node
        let start_time = Instant::now();
        let connection = endpoint
            .connect(node_id, iroh_blobs::protocol::ALPN)
            .await
            .map_err(|e| ProbeError::ConnectionFailed(e.to_string()))?;

        // Perform the probe (complete probe only)
        let stats = probe_complete(&connection, &node_id, &content)
            .await
            .map_err(|e| ProbeError::ProbeFailed(e.to_string()))?;

        let elapsed = start_time.elapsed();

        Ok(ProbeResult {
            node_id,
            content,
            stats,
            elapsed,
        })
    }
}

async fn probe_complete(
    connection: &iroh::endpoint::Connection,
    host: &NodeId,
    content: &HashAndFormat,
) -> anyhow::Result<Stats> {
    let cap = format!("{} at {}", content, host);
    let HashAndFormat { hash, format } = content;
    
    // Create a Send-compatible RNG
    let mut rng = rand::rngs::StdRng::from_entropy();
    
    match format {
        BlobFormat::Raw => {
            let size = get_or_insert_size(connection, hash).await?;
            let random_chunk = rng.gen_range(0..bao_tree::ChunkNum::chunks(size).0);
            tracing::debug!("Chunk probing {}, chunk {}", cap, random_chunk);
            let stats = chunk_probe(connection, hash, bao_tree::ChunkNum(random_chunk)).await?;
            tracing::debug!(
                "Chunk probed {}, chunk {}, {:.6}s",
                cap,
                random_chunk,
                stats.elapsed.as_secs_f64()
            );
            Ok(stats)
        }
        BlobFormat::HashSeq => {
            let (hs, sizes) = get_hash_seq_and_sizes(connection, hash, 1024 * 1024 * 100).await?;
            let ranges = random_hash_seq_ranges(&sizes, rng);
            let text = ranges
                .iter_non_empty()
                .map(|(index, ranges)| {
                    format!("child={}, ranges={:?}", index, ranges.to_chunk_ranges())
                })
                .collect::<Vec<_>>()
                .join(", ");
            tracing::debug!("Seq probing {} using {}", cap, text);
            let request = GetRequest::new(*hash, ranges);
            let request = iroh_blobs::get::fsm::start(connection.clone(), request);
            let connected = request.next().await?;
            let iroh_blobs::get::fsm::ConnectedNext::StartChild(child) =
                connected.next().await?
            else {
                unreachable!("request does not include root");
            };
            let index =
                usize::try_from(child.child_offset()).expect("child offset too large");
            let hash = hs.get(index).expect("request inconsistent with hash seq");
            let at_blob_header = child.next(hash);
            let at_end_blob = at_blob_header.drain().await?;
            let EndBlobNext::Closing(closing) = at_end_blob.next() else {
                unreachable!("request contains only one blob");
            };
            let stats = closing.next().await?;
            tracing::debug!(
                "Seq probed {} using {}, {:.6}s",
                cap,
                text,
                stats.elapsed.as_secs_f64()
            );
            Ok(stats)
        }
    }
}

async fn get_or_insert_size(
    connection: &iroh::endpoint::Connection,
    hash: &Hash,
) -> anyhow::Result<u64> {
    let (size, _) = verified_size(connection, hash).await?;
    Ok(size)
}

// async fn verified_size(
//     connection: &iroh::endpoint::Connection,
//     hash: &Hash,
// ) -> anyhow::Result<(u64, Stats)> {
//     let request = iroh_blobs::protocol::GetRequest::new(*hash, iroh_blobs::protocol::RangeSpec::default());
//     let request = iroh_blobs::get::fsm::start(connection.clone(), request);
//     let connected = request.next().await?;
//     let at_blob_header = connected.next().await?;
//     let size = at_blob_header.size();
//     let at_end_blob = at_blob_header.drain().await?;
//     let EndBlobNext::Closing(closing) = at_end_blob.next() else {
//         unreachable!("request contains only one blob");
//     };
//     let stats = closing.next().await?;
//     Ok((size, stats))
// }

pub async fn verified_size(
    connection: &iroh::endpoint::Connection,
    hash: &Hash,
) -> anyhow::Result<(u64, Stats)> {
    tracing::debug!("Getting verified size of {}", hash.to_hex());
    let request = iroh_blobs::protocol::GetRequest::new(
        *hash,
        RangeSpecSeq::from_ranges(vec![ChunkRanges::from(ChunkNum(u64::MAX)..)]),
    );
    let request = iroh_blobs::get::fsm::start(connection.clone(), request);
    let connected = request.next().await?;
    let iroh_blobs::get::fsm::ConnectedNext::StartRoot(start) = connected.next().await? else {
        unreachable!("expected start root");
    };
    let header = start.next();
    let (mut curr, size) = header.next().await?;
    let end = loop {
        match curr.next().await {
            BlobContentNext::More((next, res)) => {
                let _ = res?;
                curr = next;
            }
            BlobContentNext::Done(end) => {
                break end;
            }
        }
    };
    let EndBlobNext::Closing(closing) = end.next() else {
        unreachable!("expected closing");
    };
    let stats = closing.next().await?;
    tracing::debug!(
        "Got verified size of {}, {:.6}s",
        hash.to_hex(),
        stats.elapsed.as_secs_f64()
    );
    Ok((size, stats))
}

pub async fn chunk_probe(
    connection: &iroh::endpoint::Connection,
    hash: &Hash,
    chunk: ChunkNum,
) -> anyhow::Result<Stats> {
    let ranges = ChunkRanges::from(chunk..chunk + 1);
    let ranges = RangeSpecSeq::from_ranges([ranges]);
    let request = GetRequest::new(*hash, ranges);
    let request = iroh_blobs::get::fsm::start(connection.clone(), request);
    let connected = request.next().await?;
    let iroh_blobs::get::fsm::ConnectedNext::StartRoot(start) = connected.next().await? else {
        unreachable!("query includes root");
    };
    let header = start.next();
    let (mut curr, _size) = header.next().await?;
    let end = loop {
        match curr.next().await {
            BlobContentNext::More((next, res)) => {
                res?;
                curr = next;
            }
            BlobContentNext::Done(end) => {
                break end;
            }
        }
    };
    let EndBlobNext::Closing(closing) = end.next() else {
        unreachable!("query contains only one blob");
    };
    let stats = closing.next().await?;
    Ok(stats)
}

pub async fn get_hash_seq_and_sizes(
    connection: &iroh::endpoint::Connection,
    hash: &Hash,
    max_size: u64,
) -> anyhow::Result<(HashSeq, Arc<[u64]>)> {
    let content = HashAndFormat::hash_seq(*hash);
    tracing::debug!("Getting hash seq and children sizes of {}", content);
    let request = iroh_blobs::protocol::GetRequest::new(
        *hash,
        RangeSpecSeq::from_ranges_infinite([
            ChunkRanges::all(),
            ChunkRanges::from(ChunkNum(u64::MAX)..),
        ]),
    );
    let at_start = iroh_blobs::get::fsm::start(connection.clone(), request);
    let at_connected = at_start.next().await?;
    let iroh_blobs::get::fsm::ConnectedNext::StartRoot(start) = at_connected.next().await? else {
        unreachable!("query includes root");
    };
    let at_start_root = start.next();
    let (at_blob_content, size) = at_start_root.next().await?;
    // check the size to avoid parsing a maliciously large hash seq
    if size > max_size {
        anyhow::bail!("size too large");
    }
    let (mut curr, hash_seq) = at_blob_content.concatenate_into_vec().await?;
    let hash_seq = HashSeq::try_from(Bytes::from(hash_seq))?;
    let mut sizes = Vec::with_capacity(hash_seq.len());
    let closing = loop {
        match curr.next() {
            EndBlobNext::MoreChildren(more) => {
                let hash = match hash_seq.get(sizes.len()) {
                    Some(hash) => hash,
                    None => break more.finish(),
                };
                let at_header = more.next(hash);
                let (at_content, size) = at_header.next().await?;
                let next = at_content.drain().await?;
                sizes.push(size);
                curr = next;
            }
            EndBlobNext::Closing(closing) => break closing,
        }
    };
    let _stats = closing.next().await?;
    tracing::debug!(
        "Got hash seq and children sizes of {}: {:?}",
        content,
        sizes
    );
    Ok((hash_seq, sizes.into()))
}

/// Given a sequence of sizes of children, generate a range spec that selects a
/// random chunk of a random child.
///
/// The random chunk is chosen uniformly from the chunks of the children, so
/// larger children are more likely to be selected.
pub fn random_hash_seq_ranges(sizes: &[u64], mut rng: impl Rng + Send) -> RangeSpecSeq {
    let total_chunks = sizes
        .iter()
        .map(|size| ChunkNum::full_chunks(*size).0)
        .sum::<u64>();
    let random_chunk = rng.gen_range(0..total_chunks);
    let mut remaining = random_chunk;
    let mut ranges = vec![];
    ranges.push(ChunkRanges::empty());
    for size in sizes.iter() {
        let chunks = ChunkNum::full_chunks(*size).0;
        if remaining < chunks {
            ranges.push(ChunkRanges::from(
                ChunkNum(remaining)..ChunkNum(remaining + 1),
            ));
            break;
        } else {
            remaining -= chunks;
            ranges.push(ChunkRanges::empty());
        }
    }
    RangeSpecSeq::from_ranges(ranges)
}