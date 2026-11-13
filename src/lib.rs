//! purgar — limpador de cache e arquivos temporários para Linux.
//!
//! A biblioteca concentra a lógica de categorias, varredura, limpeza,
//! configuração e utilidades. O binário (`src/main.rs`) expõe a CLI e a TUI.

pub mod categories;
pub mod cleaner;
pub mod config;
pub mod fsutil;
pub mod model;
pub mod seguranca;
pub mod tools;
pub mod tui;
pub mod util;
