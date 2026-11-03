//! Filtered reads of `activity_log` (the orchestrator timeline). Writes live in
//! `ops_orchestrator::insert_activity_log_row`.

use crate::store::types::StoreError;

/// Filter for [`crate::VoxDb::query_activity_log`]. Every `Some` field narrows the
/// result (AND); `before_id` pages backwards from an `id` the caller already has.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityLogFilter {
    pub agent_id: Option<String>,
    pub kind: Option<String>,
    pub session_id: Option<String>,
    pub limit: u32,
    pub before_id: Option<i64>,
}

/// One `activity_log` row, newest first in query results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityLogEntry {
    pub id: i64,
    pub ts_ms: i64,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub kind: String,
    pub summary: String,
    pub detail_json: String,
}

impl crate::VoxDb {
    /// Filtered `activity_log` read, newest first, at most `filter.limit` rows.
    pub async fn query_activity_log(
        &self,
        filter: &ActivityLogFilter,
    ) -> Result<Vec<ActivityLogEntry>, StoreError> {
        let mut clauses: Vec<&str> = Vec::new();
        let mut params: Vec<turso::Value> = Vec::new();
        if let Some(agent_id) = &filter.agent_id {
            clauses.push("agent_id = ?");
            params.push(agent_id.clone().into());
        }
        if let Some(kind) = &filter.kind {
            clauses.push("kind = ?");
            params.push(kind.clone().into());
        }
        if let Some(session_id) = &filter.session_id {
            clauses.push("session_id = ?");
            params.push(session_id.clone().into());
        }
        if let Some(before_id) = filter.before_id {
            clauses.push("id < ?");
            params.push(before_id.into());
        }
        let where_clause = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };
        params.push(i64::from(filter.limit).into());
        let sql = format!(
            "SELECT id, ts_ms, agent_id, session_id, kind, summary, detail_json
             FROM activity_log {where_clause} ORDER BY id DESC LIMIT ?"
        );
        let mut rows = self.conn.query(&sql, params).await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(ActivityLogEntry {
                id: r.get(0)?,
                ts_ms: r.get(1)?,
                agent_id: r.get(2)?,
                session_id: r.get(3)?,
                kind: r.get(4)?,
                summary: r.get(5)?,
                detail_json: r.get(6)?,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::ActivityLogFilter;
    use crate::{DbConfig, VoxDb};

    async fn seeded() -> VoxDb {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
        let rows = [
            (1, Some("A1"), Some("s1"), "TaskCompleted"),
            (2, Some("A1"), Some("s2"), "TaskStarted"),
            (3, Some("A2"), Some("s1"), "TaskCompleted"),
            (4, None, None, "Note"),
        ];
        for (ts, agent, session, kind) in rows {
            db.insert_activity_log_row(ts, agent, session, kind, "sum", "{}")
                .await
                .expect("insert");
        }
        db
    }

    fn filter(limit: u32) -> ActivityLogFilter {
        ActivityLogFilter {
            limit,
            ..Default::default()
        }
    }

    async fn ts_of(db: &VoxDb, f: &ActivityLogFilter) -> Vec<i64> {
        db.query_activity_log(f)
            .await
            .expect("query")
            .into_iter()
            .map(|e| e.ts_ms)
            .collect()
    }

    #[tokio::test]
    async fn unfiltered_is_newest_first_and_limited() {
        let db = seeded().await;
        assert_eq!(ts_of(&db, &filter(10)).await, vec![4, 3, 2, 1]);
        assert_eq!(ts_of(&db, &filter(2)).await, vec![4, 3]);
    }

    #[tokio::test]
    async fn every_filter_field_narrows_the_result() {
        let db = seeded().await;
        let by_agent = ActivityLogFilter {
            agent_id: Some("A1".into()),
            ..filter(10)
        };
        assert_eq!(ts_of(&db, &by_agent).await, vec![2, 1]);
        let by_kind = ActivityLogFilter {
            kind: Some("TaskCompleted".into()),
            ..filter(10)
        };
        assert_eq!(ts_of(&db, &by_kind).await, vec![3, 1]);
        let by_session = ActivityLogFilter {
            session_id: Some("s1".into()),
            ..filter(10)
        };
        assert_eq!(ts_of(&db, &by_session).await, vec![3, 1]);
        let all = db.query_activity_log(&filter(10)).await.expect("query");
        let before = ActivityLogFilter {
            before_id: Some(all[1].id),
            ..filter(10)
        };
        assert_eq!(ts_of(&db, &before).await, vec![2, 1]);
        let combined = ActivityLogFilter {
            agent_id: Some("A1".into()),
            kind: Some("TaskCompleted".into()),
            session_id: Some("s1".into()),
            ..filter(10)
        };
        assert_eq!(ts_of(&db, &combined).await, vec![1]);
    }

    #[tokio::test]
    async fn nullable_columns_round_trip() {
        let db = seeded().await;
        let newest = &db.query_activity_log(&filter(1)).await.expect("query")[0];
        assert_eq!(newest.agent_id, None);
        assert_eq!(newest.session_id, None);
        assert_eq!(newest.kind, "Note");
        assert_eq!(newest.detail_json, "{}");
    }
}
