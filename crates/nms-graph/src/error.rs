use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GraphError {
    #[error("system not found: {0}")]
    SystemNotFound(String),

    #[error("base not found: {0}")]
    BaseNotFound(String),

    #[error("no player position available")]
    NoPlayerPosition,

    #[error("no fleet data in this save")]
    NoFleet,

    #[error("no inventory data in this save")]
    NoHoldings,

    #[error("no container matches \"{0}\"")]
    ContainerNotFound(String),
}
