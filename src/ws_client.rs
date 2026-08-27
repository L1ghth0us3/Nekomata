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
use crate::model::{AppEvent, LimitBreakSummary};
use crate::parse::{parse_combat_data, parse_logline_for_lb, ParsedLogLineEvent};

pub async fn run(ws_url: String, tx: UnboundedSender<AppEvent>, history: RecorderHandle) {
    let mut current_lb: Option<LimitBreakSummary> = None;
    let mut prev_encounter: Option<crate::model::EncounterSummary> = None;
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
                                        current_lb = None;
                                    }
                                    history.record_components(
                                        enc.clone(),
                                        rows.clone(),
                                        val,
                                        current_lb.clone(),
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
                                } else if let Some(lb_event) = parse_logline_for_lb(&val) {
                                    match lb_event {
                                        ParsedLogLineEvent::LimitBreakCast(cast) => {
                                            current_lb = Some(LimitBreakSummary {
                                                user: cast.source_name.clone(),
                                                damage: 0,
                                            });
                                            if tx.send(AppEvent::LimitBreakCast { cast }).is_err() {
                                                warn!("receiver dropped lb cast update");
                                                break;
                                            }
                                        }
                                        ParsedLogLineEvent::LimitBreakHit(hit) => {
                                            if let Some(lb) = current_lb.as_mut() {
                                                lb.damage = lb.damage.saturating_add(hit.damage);
                                            }
                                            if tx.send(AppEvent::LimitBreakHit { hit }).is_err() {
                                                warn!("receiver dropped lb hit update");
                                                break;
                                            }
                                        }
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
                current_lb = None;
                if tx.send(AppEvent::Disconnected).is_err() {
                    debug!("receiver dropped disconnected event");
                }
            }
        }

        // Backoff before reconnect
        sleep(Duration::from_secs(1)).await;
    }
}

fn should_reset_lb(
    prev: Option<&crate::model::EncounterSummary>,
    next: &crate::model::EncounterSummary,
) -> bool {
    let Some(prev) = prev else {
        return !next.is_active;
    };
    if !next.is_active {
        return false;
    }
    if !prev.is_active {
        return true;
    }
    let prev_secs = parse_duration_seconds(&prev.duration).unwrap_or(0);
    let next_secs = parse_duration_seconds(&next.duration).unwrap_or(0);
    if next_secs + 2 < prev_secs {
        return true;
    }
    let prev_damage = prev.damage.replace(',', "").parse::<f64>().unwrap_or(0.0);
    let next_damage = next.damage.replace(',', "").parse::<f64>().unwrap_or(0.0);
    next_damage + 1.0 < prev_damage
}

fn parse_duration_seconds(s: &str) -> Option<u64> {
    let mut it = s.split(':');
    let m = it.next()?.parse::<u64>().ok()?;
    let sec = it.next()?.parse::<u64>().ok()?;
    Some(m.saturating_mul(60).saturating_add(sec))
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
