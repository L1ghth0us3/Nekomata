use std::sync::{Arc, RwLock};

use tokio::sync::{mpsc, RwLock as TokioRwLock};

use crate::config;
use crate::dungeon::DungeonCatalog;
use crate::model::AppEvent;

use super::recorder::{spawn_recorder, RecorderHandle};
use super::retention::HistoryRetentionPolicy;
use super::settings_controller::RetentionState;
use super::store::HistoryStore;

/// Manages optional live history recording and sled access.
pub struct HistorySession {
    store: Option<Arc<HistoryStore>>,
    recorder: Option<RecorderHandle>,
    enabled: bool,
    dungeon_mode_enabled: bool,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    dungeon_catalog: Option<Arc<DungeonCatalog>>,
    retention_state: Arc<RwLock<RetentionState>>,
}

impl HistorySession {
    pub fn new(
        event_tx: mpsc::UnboundedSender<AppEvent>,
        dungeon_catalog: Option<Arc<DungeonCatalog>>,
        dungeon_mode_enabled: bool,
        retention_state: Arc<RwLock<RetentionState>>,
    ) -> Self {
        Self {
            store: None,
            recorder: None,
            enabled: false,
            dungeon_mode_enabled,
            event_tx,
            dungeon_catalog,
            retention_state,
        }
    }

    pub fn store(&self) -> Option<Arc<HistoryStore>> {
        self.store.clone()
    }

    pub fn recorder(&self) -> Option<RecorderHandle> {
        self.recorder.clone()
    }

    pub fn set_dungeon_mode_enabled(&mut self, enabled: bool) {
        if let Some(recorder) = &self.recorder {
            recorder.set_dungeon_mode_enabled(enabled);
        }
        self.dungeon_mode_enabled = enabled;
    }

    pub async fn enable(&mut self) -> anyhow::Result<()> {
        if self.enabled {
            return Ok(());
        }
        let store = Arc::new(HistoryStore::open_default()?);
        let recorder = spawn_recorder(
            store.clone(),
            self.event_tx.clone(),
            self.dungeon_catalog.clone(),
            self.dungeon_mode_enabled,
            Arc::clone(&self.retention_state),
        );
        self.store = Some(store);
        self.recorder = Some(recorder);
        self.enabled = true;
        Ok(())
    }

    pub async fn disable(&mut self) {
        if !self.enabled {
            return;
        }
        if let Some(recorder) = self.recorder.take() {
            recorder.flush();
            recorder.shutdown().await;
        }
        self.store = None;
        self.enabled = false;
    }

    pub async fn shutdown(&mut self) {
        if let Some(recorder) = self.recorder.take() {
            recorder.shutdown().await;
        }
        self.store = None;
        self.enabled = false;
    }

    pub fn live_db_size_bytes(&self) -> Option<u64> {
        self.store.as_ref().and_then(|s| s.size_on_disk().ok())
    }

    pub async fn delete_live_and_reopen(&mut self, reopen: bool) -> anyhow::Result<()> {
        self.disable().await;
        super::archives::delete_live_history()?;
        if reopen {
            self.enable().await?;
        }
        Ok(())
    }
}

/// Cloneable handle for WS client and other producers.
#[derive(Clone)]
pub struct HistorySessionHandle {
    inner: Arc<TokioRwLock<HistorySession>>,
}

impl HistorySessionHandle {
    pub fn new(session: HistorySession) -> Self {
        Self {
            inner: Arc::new(TokioRwLock::new(session)),
        }
    }

    pub async fn enable(&self) -> anyhow::Result<()> {
        self.inner.write().await.enable().await
    }

    pub async fn disable(&self) -> anyhow::Result<()> {
        self.inner.write().await.disable().await;
        Ok(())
    }

    pub async fn shutdown(&self) {
        self.inner.write().await.shutdown().await;
    }

    pub async fn store(&self) -> Option<Arc<HistoryStore>> {
        self.inner.read().await.store()
    }

    pub async fn set_dungeon_mode_enabled(&self, enabled: bool) {
        self.inner.write().await.set_dungeon_mode_enabled(enabled);
    }

    pub async fn record_components(
        &self,
        encounter: crate::model::EncounterSummary,
        rows: Vec<crate::model::CombatantRow>,
        raw: serde_json::Value,
        lb_summary: Option<crate::model::LimitBreakSummary>,
    ) {
        if let Some(recorder) = self.inner.read().await.recorder() {
            recorder.record_components(encounter, rows, raw, lb_summary);
        }
    }

    pub async fn flush(&self) {
        if let Some(recorder) = self.inner.read().await.recorder() {
            recorder.flush();
        }
    }

    pub async fn cut_dungeon_session(&self) {
        if let Some(recorder) = self.inner.read().await.recorder() {
            recorder.cut_dungeon_session();
        }
    }

    pub async fn live_db_size_bytes(&self) -> Option<u64> {
        self.inner.read().await.live_db_size_bytes()
    }

    pub async fn apply_retention_if_applied(
        &self,
        policy: &HistoryRetentionPolicy,
        cfg: &config::AppConfig,
    ) -> anyhow::Result<()> {
        if !policy.is_applied_in_config(cfg) {
            return Ok(());
        }
        if let Some(store) = self.store().await {
            store.apply_retention(policy)?;
        }
        Ok(())
    }

    pub async fn delete_live_and_reopen(&self, reopen: bool) -> anyhow::Result<()> {
        self.inner
            .write()
            .await
            .delete_live_and_reopen(reopen)
            .await
    }
}
