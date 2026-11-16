//! Streaming SSE feed, paginated claim/source views, and content-addressed chunk retrieval (`STRM-01`, `STRM-02`).

use std::convert::Infallible;
use std::net::SocketAddr;

use axum::Router;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode as SC, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::broadcast::error::RecvError;
use vox_search::chunk_spill::{
    ChunkSpillStore, SPILL_THRESHOLD_BYTES, SpillPointer, is_valid_digest_hex,
};

use super::research::artifact_mirror::ResearchRunArtifact;
use super::research::extract_research_summary;
use super::{api_db, enforce_route};
use crate::http_gateway::GatewayState;
use crate::http_gateway::scientia_feed::ReplayWindow;
use crate::services::routes::{err, ok};

const SURFACES: &[&str] = &["research"];
const MAX_LIMIT: u32 = 200;

#[derive(Debug, Deserialize, Default)]
pub struct EventsQuery {
    #[serde(default)]
    pub from_seq: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PageQuery {
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub cursor: Option<i64>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Clone, Copy)]
enum Kind {
    Claims,
    Sources,
}

pub(super) fn sha3_256_hex(bytes: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::compliance_hash(bytes))
}

pub(super) fn maybe_spill_bytes(session_id: i64, bytes: &[u8]) -> Option<SpillPointer> {
    if bytes.len() <= SPILL_THRESHOLD_BYTES {
        return None;
    }
    let digest = sha3_256_hex(bytes);
    let mut ptr = ChunkSpillStore::default()
        .spill_with_digest(&digest, bytes)
        .ok()??;
    ptr.chunk_url = Some(format!(
        "/api/v2/research/sessions/{session_id}/chunks/{digest}"
    ));
    Some(ptr)
}

fn sse_ev(name: &str, id: u64, data: String) -> Event {
    Event::default().event(name).id(id.to_string()).data(data)
}

fn sse_resync(sid: &str, from: Option<u64>, old: u64, latest: u64, skip: Option<u64>) -> Event {
    let body = json!({
        "kind": "resync_required", "session_id": sid, "requested_from_seq": from,
        "oldest_available_seq": old, "latest_seq": latest, "skipped": skip,
    });
    sse_ev("resync_required", latest, body.to_string())
}

fn fail(s: SC, code: &str, msg: &str) -> Response {
    (s, err(code, msg)).into_response()
}

/// GET /api/v2/research/sessions/{id}/events?from_seq=N
pub async fn get_research_session_events(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(session_id): Path<i64>,
    Query(q): Query<EventsQuery>,
) -> Response {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SURFACES) {
        return e.into_response();
    }
    let hub = gs.research_feed().clone();
    hub.ensure_bridge_started(gs.server_state());
    let mut rx = hub.subscribe_deltas();
    let hdr = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    let from_seq = q.from_seq.or(hdr);
    let skey = session_id.to_string();
    let replay = hub.replay_since(&skey, from_seq);
    let stream = async_stream::stream! {
        let mut last = from_seq.unwrap_or(0);
        match replay {
            ReplayWindow::ResyncRequired { oldest_available_seq, latest_seq } => {
                last = latest_seq;
                yield Ok::<Event, Infallible>(sse_resync(&skey, from_seq, oldest_available_seq, latest_seq, None));
            }
            ReplayWindow::Events(events) => {
                for ev in events {
                    last = last.max(ev.seq);
                    let raw = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
                    yield Ok::<Event, Infallible>(sse_ev("research.session.delta", ev.seq, raw));
                }
            }
        }
        loop {
            match rx.recv().await {
                Ok(ev) if ev.seq > last && ev.session_id == skey => {
                    last = ev.seq;
                    let raw = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
                    yield Ok::<Event, Infallible>(sse_ev("research.session.delta", ev.seq, raw));
                }
                Ok(_) => {}
                Err(RecvError::Lagged(skip)) => {
                    let latest = hub.latest_seq();
                    let old = hub.oldest_seq().unwrap_or(latest);
                    yield Ok::<Event, Infallible>(sse_resync(&skey, Some(last), old, latest, Some(skip)));
                    last = latest;
                }
                Err(RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

fn pack_page<T: serde::Serialize>(
    tot: i64,
    next: Option<i64>,
    items: Vec<T>,
) -> (usize, Option<i64>, Value, usize) {
    let n = items.len();
    (
        tot.max(0) as usize,
        next,
        serde_json::to_value(items).unwrap_or_default(),
        n,
    )
}

async fn db_page(
    db: &vox_db::VoxDb,
    kind: Kind,
    sid: i64,
    after: Option<i64>,
    lim: u32,
) -> Result<(usize, Option<i64>, Value, usize), String> {
    if matches!(kind, Kind::Claims) {
        let p = db
            .list_session_claims_page(sid, after, lim)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(pack_page(p.total_count, p.next_cursor, p.items));
    }
    let p = db
        .list_session_sources_page(sid, after, lim)
        .await
        .map_err(|e| e.to_string())?;
    Ok(pack_page(p.total_count, p.next_cursor, p.items))
}

fn artifact_slice(kind: Kind, raw: &str, off: usize, lim: usize) -> (usize, Value, usize) {
    if matches!(kind, Kind::Claims) {
        let all = extract_research_summary(raw).claims;
        let tot = all.len();
        let slice: Vec<_> = all.into_iter().skip(off).take(lim).collect();
        return (
            tot,
            serde_json::to_value(&slice).unwrap_or_default(),
            slice.len(),
        );
    }
    let parsed: ResearchRunArtifact = serde_json::from_str(raw).unwrap_or_default();
    let tot = parsed.result.citations.len();
    let slice: Vec<Value> = parsed.result.citations.into_iter().skip(off).take(lim).map(|c| {
        json!({"source_id": c.source_id, "url": c.url, "title": c.title, "snippet": c.snippet, "confidence": c.confidence})
    }).collect();
    let n = slice.len();
    (tot, Value::Array(slice), n)
}

async fn paginate(
    gs: &GatewayState,
    peer: &SocketAddr,
    hdrs: &HeaderMap,
    sid: i64,
    q: PageQuery,
    kind: Kind,
) -> Json<Value> {
    if let Err(e) = enforce_route(gs, peer, hdrs, SURFACES) {
        return e;
    }
    let db = match api_db(gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let (key, def) = match kind {
        Kind::Claims => ("claims", 100),
        Kind::Sources => ("sources", 50),
    };
    let (lim, off) = (
        q.limit.unwrap_or(def).clamp(1, MAX_LIMIT),
        q.offset.unwrap_or(0),
    );
    let mut after = q.cursor;
    let mut rem = if after.is_none() { off } else { 0 };
    while rem > 0 {
        let step = rem.min(MAX_LIMIT as usize) as u32;
        let Ok((_, next, _, n)) = db_page(&db, kind, sid, after, step).await else {
            break;
        };
        after = next;
        if n == 0 || after.is_none() {
            break;
        }
        rem = rem.saturating_sub(n);
    }
    let (src, tot, next_c, items, count) = match db_page(&db, kind, sid, after, lim).await {
        Ok((tot, next, items, n)) if tot > 0 || q.cursor.is_some() => ("db", tot, next, items, n),
        Ok(_) => match db.get_research_artifact(sid).await {
            Ok(Some(a)) => {
                let (t, i, n) = artifact_slice(kind, &a.artifact_json, off, lim as usize);
                ("artifact", t, None, i, n)
            }
            Ok(None) => return err("not_found", &format!("session {sid} {key} not found")),
            Err(e) => return err("db_error", &e.to_string()),
        },
        Err(e) => return err("db_error", &e),
    };
    let next_off = off.saturating_add(count);
    let more = next_off < tot && (src != "db" || next_c.is_some());
    let mut obj = json!({
        "session_id": sid, "source": src, "offset": off, "limit": lim, "total_count": tot,
        "next_cursor": next_c, "next_offset": more.then_some(next_off), "has_more": more,
    });
    obj.as_object_mut().unwrap().insert(key.into(), items);
    ok(obj)
}

/// GET /api/v2/research/sessions/{id}/claims?offset=0&limit=100
pub async fn get_research_session_claims(
    State(gs): State<GatewayState>,
    c: ConnectInfo<SocketAddr>,
    h: HeaderMap,
    Path(sid): Path<i64>,
    Query(q): Query<PageQuery>,
) -> Json<Value> {
    paginate(&gs, &c.0, &h, sid, q, Kind::Claims).await
}

/// GET /api/v2/research/sessions/{id}/sources?offset=0&limit=50
pub async fn get_research_session_sources(
    State(gs): State<GatewayState>,
    c: ConnectInfo<SocketAddr>,
    h: HeaderMap,
    Path(sid): Path<i64>,
    Query(q): Query<PageQuery>,
) -> Json<Value> {
    paginate(&gs, &c.0, &h, sid, q, Kind::Sources).await
}

/// GET /api/v2/research/sessions/{id}/chunks/{hash}
pub async fn get_research_session_chunk(
    State(gs): State<GatewayState>,
    c: ConnectInfo<SocketAddr>,
    h: HeaderMap,
    Path((sid, hash)): Path<(i64, String)>,
) -> Response {
    if let Err(e) = enforce_route(&gs, &c.0, &h, SURFACES) {
        return e.into_response();
    }
    if !is_valid_digest_hex(&hash) {
        return fail(SC::BAD_REQUEST, "invalid_digest", "bad chunk hash");
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e.into_response(),
    };
    match db.get_research_session(sid).await {
        Ok(Some(_)) => {}
        Ok(None) => return fail(SC::NOT_FOUND, "not_found", "session not found"),
        Err(e) => return fail(SC::INTERNAL_SERVER_ERROR, "db_error", &e.to_string()),
    }
    match ChunkSpillStore::default().read_chunk(&hash) {
        Ok(Some(b)) if sha3_256_hex(&b) == hash => {
            let ct = if b.first().copied() == Some(b'{') {
                "application/json; charset=utf-8"
            } else {
                "text/plain; charset=utf-8"
            };
            (SC::OK, [(header::CONTENT_TYPE, ct)], b).into_response()
        }
        Ok(Some(_)) => fail(
            SC::INTERNAL_SERVER_ERROR,
            "chunk_corrupt",
            "digest mismatch",
        ),
        Ok(None) => fail(SC::NOT_FOUND, "not_found", "chunk not found"),
        Err(e) => fail(SC::BAD_REQUEST, "chunk_error", &e.to_string()),
    }
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new().nest(
        "/research/sessions/{id}",
        Router::new()
            .route("/events", get(get_research_session_events))
            .route("/claims", get(get_research_session_claims))
            .route("/sources", get(get_research_session_sources))
            .route("/chunks/{hash}", get(get_research_session_chunk)),
    )
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use vox_research_events::ResearchEvent;

    use super::*;
    use crate::gui_ops::test_support::{gateway_with_memory_db, peer};

    async fn sse(gs: &GatewayState, seq: u64) -> Response {
        let q = Query(EventsQuery {
            from_seq: Some(seq),
        });
        get_research_session_events(State(gs.clone()), peer(), HeaderMap::new(), Path(77), q).await
    }

    async fn chk(gs: &GatewayState, sid: i64, h: &str) -> Response {
        let p = Path((sid, h.to_string()));
        get_research_session_chunk(State(gs.clone()), peer(), HeaderMap::new(), p).await
    }

    #[tokio::test]
    async fn sse_replay_resync_and_chunk_guard() {
        let (gs, db) = gateway_with_memory_db().await;
        for i in 1..=3u8 {
            let e = ResearchEvent::WaveStarted {
                session_id: "77".into(),
                wave: i,
                max_waves: 3,
                tier: "deep".into(),
                subquery_count: 2,
            };
            gs.research_feed().publish_research_event(e, i as i64);
        }
        let resp = sse(&gs, 1).await;
        assert_eq!(resp.status(), SC::OK);
        let mut s = resp.into_body().into_data_stream();
        let mut out = String::new();
        for _ in 0..2 {
            let b = tokio::time::timeout(std::time::Duration::from_secs(1), s.next()).await;
            out.push_str(&String::from_utf8_lossy(&b.unwrap().unwrap().unwrap()));
        }
        assert!(
            out.contains("event: research.session.delta")
                && out.contains("id: 2")
                && out.contains("id: 3")
        );
        let f = sse(&gs, 99)
            .await
            .into_body()
            .into_data_stream()
            .next()
            .await
            .unwrap()
            .unwrap();
        assert!(String::from_utf8_lossy(&f).contains("event: resync_required"));

        let sid = db.create_research_session("s-chunk", "q").await.unwrap();
        assert_eq!(chk(&gs, sid, "../etc").await.status(), SC::BAD_REQUEST);
        let payload = vec![b'x'; SPILL_THRESHOLD_BYTES + 64];
        let ptr = maybe_spill_bytes(sid, &payload).expect("spilled");
        let ok = chk(&gs, sid, &ptr.content_hash).await;
        assert_eq!(ok.status(), SC::OK);
        let got = axum::body::to_bytes(ok.into_body(), 1 << 20).await.unwrap();
        assert_eq!(got.len(), payload.len());
    }
}
