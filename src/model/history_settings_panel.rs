use crate::history::{default_backup_name, ArchiveEntry, HistoryLimitKind, HistoryRetentionPolicy};

/// Which row is selected in the History Settings overlay.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum HistorySettingsField {
    #[default]
    Recording,
    LimitKind,
    LimitValue,
    Separator,
    CreateBackup,
    BrowseArchives,
    DeleteCurrent,
}

impl HistorySettingsField {
    pub fn is_selectable(self) -> bool {
        !matches!(self, Self::Separator)
    }

    pub fn next(mut self, limit_kind: HistoryLimitKind) -> Self {
        loop {
            self = match self {
                Self::Recording => Self::LimitKind,
                Self::LimitKind => {
                    if limit_kind == HistoryLimitKind::None {
                        Self::CreateBackup
                    } else {
                        Self::LimitValue
                    }
                }
                Self::LimitValue => Self::CreateBackup,
                Self::CreateBackup => Self::BrowseArchives,
                Self::BrowseArchives => Self::DeleteCurrent,
                Self::DeleteCurrent => Self::Recording,
                Self::Separator => Self::CreateBackup,
            };
            if self.is_selectable() {
                return self;
            }
        }
    }

    pub fn prev(mut self, limit_kind: HistoryLimitKind) -> Self {
        loop {
            self = match self {
                Self::Recording => Self::DeleteCurrent,
                Self::LimitKind => Self::Recording,
                Self::LimitValue => Self::LimitKind,
                Self::CreateBackup => {
                    if limit_kind == HistoryLimitKind::None {
                        Self::LimitKind
                    } else {
                        Self::LimitValue
                    }
                }
                Self::BrowseArchives => Self::CreateBackup,
                Self::DeleteCurrent => Self::BrowseArchives,
                Self::Separator => Self::LimitKind,
            };
            if self.is_selectable() {
                return self;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfirmAction {
    ApplyRetention { policy: HistoryRetentionPolicy },
    DiscardDraft,
    DeleteArchive { name: String },
    DeleteLiveHistory,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Copy)]
pub enum ConfirmFocus {
    #[default]
    Cancel,
    Confirm,
}

#[derive(Clone, Debug)]
pub struct ConfirmDialog {
    pub message: String,
    pub confirm_label: String,
    pub action: ConfirmAction,
    pub focus: ConfirmFocus,
}

#[derive(Clone, Debug, Default)]
pub struct FilenamePrompt {
    pub title: String,
    pub value: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ArchiveBrowser {
    pub entries: Vec<ArchiveEntry>,
    pub selected: usize,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HistorySettingsPanel {
    pub visible: bool,
    pub cursor: HistorySettingsField,
    pub draft_limit: HistoryRetentionPolicy,
    pub committed_limit: HistoryRetentionPolicy,
    pub live_db_size: Option<u64>,
    pub status_message: Option<String>,
    pub confirm: Option<ConfirmDialog>,
    pub filename_prompt: Option<FilenamePrompt>,
    pub archive_browser: Option<ArchiveBrowser>,
}

impl Default for HistorySettingsPanel {
    fn default() -> Self {
        Self {
            visible: false,
            cursor: HistorySettingsField::default(),
            draft_limit: HistoryRetentionPolicy::default(),
            committed_limit: HistoryRetentionPolicy::default(),
            live_db_size: None,
            status_message: None,
            confirm: None,
            filename_prompt: None,
            archive_browser: None,
        }
    }
}

impl HistorySettingsPanel {
    pub fn open(
        &mut self,
        committed: HistoryRetentionPolicy,
        live_db_size: Option<u64>,
    ) {
        self.visible = true;
        self.cursor = HistorySettingsField::Recording;
        self.committed_limit = committed.clone();
        self.draft_limit = committed;
        self.live_db_size = live_db_size;
        self.status_message = None;
        self.confirm = None;
        self.filename_prompt = None;
        self.archive_browser = None;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.confirm = None;
        self.filename_prompt = None;
        self.archive_browser = None;
    }

    pub fn has_draft_changes(&self) -> bool {
        self.draft_limit != self.committed_limit
    }

    pub fn open_filename_prompt(&mut self) {
        self.filename_prompt = Some(FilenamePrompt {
            title: "Archive name".to_string(),
            value: default_backup_name(),
            error: None,
        });
    }

    pub fn open_archive_browser(&mut self, entries: Vec<ArchiveEntry>) {
        self.archive_browser = Some(ArchiveBrowser {
            entries,
            selected: 0,
            error: None,
        });
    }
}
