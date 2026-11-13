//! 应用根状态、窗口呈现和生命周期协调。

pub(crate) mod help;
pub(crate) mod keyboard;
pub(crate) mod launch;
mod lifecycle;
pub(crate) mod links;
pub(crate) mod menu;
pub(crate) mod preferences;
mod state;
pub(crate) mod text_copy;
pub(crate) mod updates;
pub(crate) mod view;

pub(crate) use state::{AppState, setup};

pub(crate) mod files;
