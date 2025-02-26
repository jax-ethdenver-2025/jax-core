use std::error::Error;

use clap::Subcommand;

use super::ops::Init as InitOp;
use super::ops::ListBlobs as ListBlobsOp;
use super::ops::Probe as ProbeOp;
use super::ops::Serve as ServeOp;
use super::ops::Share as ShareOp;
use super::ops::Status as StatusOp;
use super::ops::Query as QueryOp;

pub use clap::Parser;

use std::fmt;

#[async_trait::async_trait]
pub trait Op: Send + Sync {
    type Error: Error + Send + Sync + 'static;
    type Output;

    async fn execute(&self) -> Result<Self::Output, Self::Error>;
}

#[macro_export]
macro_rules! command_enum {
    ($(($variant:ident, $type:ty)),* $(,)?) => {
        #[derive(Subcommand, Debug, Clone)]
        pub enum Command {
            $($variant($type),)*
        }

        #[derive(Debug)]
        pub enum OpOutput {
            $($variant(<$type as Op>::Output),)*
        }

        #[derive(Debug, thiserror::Error)]
        pub enum OpError {
            $(
                #[error(transparent)]
                $variant(<$type as Op>::Error),
            )*
        }

        #[async_trait::async_trait]
        impl Op for Command {
            type Output = OpOutput;
            type Error = OpError;

            async fn execute(&self) -> Result<Self::Output, Self::Error> {
                match self {
                    $(
                        Command::$variant(op) => {
                            op.execute().await
                                .map(OpOutput::$variant)
                                .map_err(OpError::$variant)
                        },
                    )*
                }
            }
        }
    };
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

use crate::command_enum;

command_enum! {
    (Init, InitOp),
    (Serve, ServeOp),
    (Status, StatusOp),
    (Share, ShareOp),
    (Query, QueryOp),
    (Probe, ProbeOp),
    (ListBlobs, ListBlobsOp),
}

impl fmt::Display for OpOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpOutput::Init(node_id) => write!(f, "device initialized with node id: {}", node_id),
            OpOutput::Serve(_) => write!(f, ""),
            OpOutput::Status(output) => write!(f, "{}", output),
            OpOutput::Share(ticket) => write!(f, "{}", ticket),
            OpOutput::Probe(result) => write!(f, "{}", result),
            OpOutput::ListBlobs(output) => write!(f, "{}", output),
            OpOutput::Query(output) => write!(f, "{}", output),
        }
    }
}
