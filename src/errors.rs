use std::borrow::Cow;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppErrorKind {
    History,
    Network,
    Storage,
    Unknown,
}

impl AppErrorKind {
    pub fn label(self) -> &'static str {
        match self {
            AppErrorKind::History => "History",
            AppErrorKind::Network => "Network",
            AppErrorKind::Storage => "Storage",
            AppErrorKind::Unknown => "Unknown",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppError {
    kind: AppErrorKind,
    message: String,
}

impl AppError {
    pub fn new(kind: AppErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> AppErrorKind {
        self.kind
    }

    /// Produce a single-line summary suitable for compact footer rendering.
    pub fn summary_line(&self) -> Cow<'_, str> {
        let collapsed = self
            .message
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if collapsed.len() <= 120 {
            Cow::Owned(collapsed)
        } else {
            let mut truncated = collapsed.chars().take(117).collect::<String>();
            truncated.push_str("...");
            Cow::Owned(truncated)
        }
    }
}
