//! Vorto's core: recording, settings and history, the voice model catalog, and the supervisor
//! that runs the recognition engine (`vorto-engine.exe`) as a separate, killable process.
pub mod audio;
pub mod data;
pub mod protocol;
pub mod provider;
pub mod supervisor;
