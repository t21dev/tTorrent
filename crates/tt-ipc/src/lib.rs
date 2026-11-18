//! The engine's typed API. The window (`tt-ui`), a second launch, the Web API
//! and the MCP server all speak it. On the wire it is length-prefixed JSON
//! over a local socket: a named pipe on Windows, a Unix socket elsewhere.

pub mod columns;
pub mod fmt;
pub mod settings;
pub mod transport;
mod types;

pub use columns::Column;
pub use settings::{DetailsTab, EngineSettings, ShareLimitAction, ShareLimitsMode, UiPrefs};
pub use types::*;

/// Bumped when a message changes shape; the window refuses an engine with another.
pub const PROTOCOL_VERSION: u32 = 1;
