use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn};

use crate::history::RecorderHandle;
use crate::lb::{should_reset_lb, LimitBreakTracker};
use crate::model::{AppEvent, EncounterSummary};
use crate::parse::{parse_combat_data, parse_logline_for_lb};

pub async fn run(ws_url: String, tx: UnboundedSender<AppEvent>, history: RecorderHandle) {
    let mut lb_tracker = LimitBreakTracker::default();
    let mut prev_encounter: Option<EncounterSummary> = None;
    // Simple reconnect loop
    loop {
        debug!(%ws_url, "websocket connect attempt");
        match connect_async(&ws_url).await {
            Ok((ws_stream, resp)) => {
                let (mut write, mut read) = ws_stream.split();
                info!(status = ?resp.status(), "websocket connected");
                let _ = tx.send(AppEvent::Connected);

                // Perform handshake: getLanguage, then subscribe
                if let Err(err) = write
                    .send(Message::Text("{\"call\":\"getLanguage\"}".to_string()))
                    .await
                {
                    warn!(error = ?err, "failed to send getLanguage call");
                }
                if let Err(err) = write
                    .send(Message::Text(
                        "{\"call\":\"subscribe\",\"events\":[\"CombatData\",\"LogLine\"]}"
                            .to_string(),
                    ))
                    .await
                {
                    warn!(error = ?err, "failed to send subscribe call");
                }

                // Reader loop
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(txt)) => match serde_json::from_str::<Value>(&txt) {
                            Ok(val) => {
                                if let Some((enc, rows)) = parse_combat_data(&val) {
                                    if should_reset_lb(prev_encounter.as_ref(), &enc) {
                                        lb_tracker.reset();
                                        let _ = tx.send(AppEvent::LimitBreakUpdate {
                                            summary: None,
                                        });
                                    }
                                    let lb_summary = lb_tracker.summary();
                                    history.record_components(
                                        enc.clone(),
                                        rows.clone(),
                                        val,
                                        lb_summary.clone(),
                                    );
                                    prev_encounter = Some(enc.clone());
                                    if tx
                                        .send(AppEvent::CombatData {
                                            encounter: enc,
                                            rows,
                                        })
                                        .is_err()
                                    {
                                        warn!("receiver dropped websocket updates");
                                        break;
                                    }
                                } else if let Some(lb_line) = parse_logline_for_lb(&val) {
                                    let summary = lb_tracker.apply_line(
                                        &lb_line.cast,
                                        &lb_line.target_id,
                                        lb_line.damage,
                                    );
                                    if tx
                                        .send(AppEvent::LimitBreakUpdate {
                                            summary: Some(summary),
                                        })
                                        .is_err()
                                    {
                                        warn!("receiver dropped lb update");
                                        break;
                                    }
                                } else {
                                    let event_type = val
                                        .get("type")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("unknown");
                                    debug!(%event_type, "ignored websocket message");
                                }
                            }
                            Err(err) => {
                                let snippet: String = txt.chars().take(128).collect();
                                warn!(error = ?err, snippet, "failed to parse websocket text frame as JSON");
                            }
                        },
                        Ok(Message::Binary(_)) => {
                            debug!("ignored binary websocket frame");
                        }
                        Ok(Message::Ping(_)) => {
                            debug!("received websocket ping");
                        }
                        Ok(Message::Pong(_)) => {
                            debug!("received websocket pong");
                        }
                        Ok(Message::Frame(_)) => {}
                        Ok(Message::Close(frame)) => {
                            log_close_frame(frame.as_ref());
                            break;
                        }
                        Err(err) => {
                            warn!(error = ?err, "websocket read error");
                            break;
                        }
                    }
                }
                history.flush();
                if tx.send(AppEvent::Disconnected).is_err() {
                    debug!("receiver dropped disconnected event");
                }
                info!("websocket loop exited, scheduling reconnect");
            }
            Err(err) => {
                warn!(error = ?err, "websocket connection failed");
                history.flush();
                prev_encounter = None;
                lb_tracker.reset();
                let _ = tx.send(AppEvent::LimitBreakUpdate { summary: None });
                if tx.send(AppEvent::Disconnected).is_err() {
                    debug!("receiver dropped disconnected event");
                }
            }
        }

        // Backoff before reconnect
        sleep(Duration::from_secs(1)).await;
    }
}

fn log_close_frame(frame: Option<&CloseFrame<'_>>) {
    if let Some(close) = frame {
        info!(
            code = ?close.code,
            reason = %close.reason,
            "websocket received close frame"
        );
    } else {
        info!("websocket closed without frame");
    }
}
