//! Refine's core: everything except the window. The Tauri app (`src-tauri`) is a thin layer of
//! commands over [`app::Core`].

pub mod app;
pub mod check;
pub mod error;
pub mod export;
pub mod insights;
pub mod llm;
pub mod model;
pub mod prompt;
pub mod secrets;
pub mod settings;
pub mod stats;
pub mod store;
pub mod taxonomy;
pub mod text;
pub mod tone;

pub use error::{AppError, Error, Result};
