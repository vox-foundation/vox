use crate::store::types::StoreError;
use turso::params;

impl crate::VoxDb {
    /// Upsert a knowledge node manually
    pub async fn upsert_knowledge_node(
        &self,
        id: &str,
        label: &str,
        content: &str,
        node_type: Option<&str>,
        metadata: Option<&str>,
        _vcs_snapshot_id: Option<&str>,
    ) -> Result<(), StoreError> {
        let id = id.to_string();
        let label = label.to_string();
        let content = content.to_string();
        let node_type = node_type.map(str::to_string);
        let metadata = metadata.map(str::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO knowledge_nodes (id, label, content, node_type, metadata)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(id) DO UPDATE SET
                         label = excluded.label,
                         content = excluded.content,
                         node_type = excluded.node_type,
                         metadata = excluded.metadata",
                    params![
                        id.as_str(),
                        label.as_str(),
                        content.as_str(),
                        node_type.as_deref(),
                        metadata.as_deref(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Create an edge between knowledge nodes
    pub async fn create_knowledge_edge(
        &self,
        source_id: &str,
        target_id: &str,
        relation: &str,
        weight: f32,
        _metadata: Option<&str>,
    ) -> Result<(), StoreError> {
        let source_id = source_id.to_string();
        let target_id = target_id.to_string();
        let relation = relation.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO knowledge_edges (src_id, dst_id, relation, weight)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(src_id, dst_id, relation) DO UPDATE SET
                         weight = excluded.weight",
                    params![
                        source_id.as_str(),
                        target_id.as_str(),
                        relation.as_str(),
                        weight
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Fetch neighboring nodes along with their relations
    pub async fn get_knowledge_neighbors(
        &self,
        node_id: &str,
    ) -> Result<Vec<(String, String, String, f32)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT e.dst_id, n.label, e.relation, e.weight
                 FROM knowledge_edges e
                 JOIN knowledge_nodes n ON e.dst_id = n.id
                 WHERE e.src_id = ?1
                 UNION
                 SELECT e.src_id, n.label, e.relation, e.weight
                 FROM knowledge_edges e
                 JOIN knowledge_nodes n ON e.src_id = n.id
                 WHERE e.dst_id = ?1",
                params![node_id],
            )
            .await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let id: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let label: String = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            let rel: String = row.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
            let w: f64 = row.get(3).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push((id, label, rel, w as f32));
        }
        Ok(out)
    }

    /// Find reachable knowledge nodes starting from `root_id` using a recursive CTE.
    ///
    /// Cycle-safe traversal tracks visited node paths using `visited_path` and verifies
    /// `instr(p.visited_path, '/' || next_id || '/') = 0` before expanding adjacent edges.
    /// Supports directions: `"forward"`, `"reverse"`, and `"undirected"`.
    pub async fn find_reachable_knowledge_nodes_cte(
        &self,
        root_id: &str,
        max_depth: usize,
        direction: &str,
    ) -> Result<Vec<String>, StoreError> {
        let dir = match direction.trim().to_ascii_lowercase().as_str() {
            "forward" => "forward",
            "reverse" => "reverse",
            "undirected" => "undirected",
            other => {
                return Err(StoreError::Db(format!(
                    "invalid graph traversal direction '{other}': expected 'forward', 'reverse', or 'undirected'"
                )));
            }
        };

        let conn = self.conn.clone();
        let breaker = self.breaker.clone();
        let r = root_id.to_string();
        let dir_str = dir.to_string();

        breaker
            .call(|| async move {
                let sql = "
                WITH RECURSIVE graph_path(node_id, depth, visited_path) AS (
                    SELECT ?1 AS node_id, 0 AS depth, '/' || ?1 || '/' AS visited_path
                    UNION ALL
                    SELECT 
                        CASE WHEN ?3 = 'reverse' THEN e.src_id 
                             WHEN ?3 = 'forward' THEN e.dst_id 
                             ELSE (CASE WHEN e.src_id = p.node_id THEN e.dst_id ELSE e.src_id END)
                        END,
                        p.depth + 1,
                        p.visited_path || (CASE WHEN ?3 = 'reverse' THEN e.src_id 
                                                WHEN ?3 = 'forward' THEN e.dst_id 
                                                ELSE (CASE WHEN e.src_id = p.node_id THEN e.dst_id ELSE e.src_id END)
                                           END) || '/'
                    FROM knowledge_edges e
                    JOIN graph_path p ON (
                        (?3 = 'forward' AND e.src_id = p.node_id) OR
                        (?3 = 'reverse' AND e.dst_id = p.node_id) OR
                        (?3 = 'undirected' AND (e.src_id = p.node_id OR e.dst_id = p.node_id))
                    )
                    WHERE p.depth < ?2
                      AND instr(p.visited_path, '/' || (
                          CASE WHEN ?3 = 'reverse' THEN e.src_id 
                               WHEN ?3 = 'forward' THEN e.dst_id 
                               ELSE (CASE WHEN e.src_id = p.node_id THEN e.dst_id ELSE e.src_id END)
                          END
                      ) || '/') = 0
                )
                SELECT DISTINCT node_id FROM graph_path;
                ";
                match conn
                    .query(sql, params![r.as_str(), max_depth as i64, dir_str.as_str()])
                    .await
                {
                    Ok(mut rows) => {
                        let mut out = Vec::new();
                        while let Some(row) = rows.next().await? {
                            let id: String =
                                row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
                            out.push(id);
                        }
                        Ok(out)
                    }
                    Err(e) if e.to_string().contains("Recursive CTE") => {
                        // Fallback when the local in-memory engine (e.g. Limbo / turso_core)
                        // lacks parser support for recursive CTEs. Preserves exact semantics.
                        find_reachable_knowledge_nodes_fallback(
                            &conn,
                            &r,
                            max_depth,
                            dir_str.as_str(),
                        )
                        .await
                    }
                    Err(e) => Err(StoreError::from(e)),
                }
            })
            .await
    }

    /// Full-text LIKE search over `knowledge_nodes` (label + content).
    ///
    /// Returns `(id, label, snippet)` — snippet is the first 200 chars of `content`.
    /// Called from `vox-db/src/lib.rs` `VoxDb::search_memories`.
    pub async fn query_knowledge_nodes(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        let lim = limit.clamp(1, 1_000);
        let use_fts = self
            .sqlite_capabilities_snapshot()
            .await
            .ok()
            .is_some_and(|p| p.fts5_reported);
        if use_fts && self.knowledge_nodes_fts_ready().await.unwrap_or(false) {
            let q = super::sanitize_fts_query(query);
            if !q.is_empty()
                && let Ok(out) = self.query_knowledge_nodes_fts(&q, lim).await
                && !out.is_empty()
            {
                return Ok(out);
            }
        }
        self.query_knowledge_nodes_like(query, lim).await
    }

    async fn knowledge_nodes_fts_ready(&self) -> Result<bool, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'knowledge_nodes_fts' LIMIT 1",
                (),
            )
            .await?;
        Ok(rows.next().await?.is_some())
    }

    async fn query_knowledge_nodes_like(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        let pat = format!("%{query}%");
        let mut rows = self
            .conn
            .query(
                "SELECT id, label, COALESCE(SUBSTR(content, 1, 200), '')
                 FROM knowledge_nodes
                 WHERE label LIKE ?1 OR content LIKE ?1
                 ORDER BY created_at DESC LIMIT ?2",
                params![pat, limit],
            )
            .await?;
        collect_knowledge_node_rows(&mut rows).await
    }

    async fn query_knowledge_nodes_fts(
        &self,
        match_query: &str,
        limit: i64,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT k.id, k.label, COALESCE(SUBSTR(k.content, 1, 200), '')
                 FROM knowledge_nodes_fts f
                 JOIN knowledge_nodes k ON k.rowid = f.rowid
                 WHERE knowledge_nodes_fts MATCH ?1
                 ORDER BY k.created_at DESC LIMIT ?2",
                params![match_query, limit],
            )
            .await?;
        collect_knowledge_node_rows(&mut rows).await
    }
}

async fn collect_knowledge_node_rows(
    rows: &mut crate::GuardedRows,
) -> Result<Vec<(String, String, String)>, StoreError> {
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        let id: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        let label: String = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
        let snippet: String = row.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
        out.push((id, label, snippet));
    }
    Ok(out)
}

async fn find_reachable_knowledge_nodes_fallback(
    conn: &crate::GuardedConnection,
    root_id: &str,
    max_depth: usize,
    direction: &str,
) -> Result<Vec<String>, StoreError> {
    use std::collections::VecDeque;

    let mut queue = VecDeque::new();
    let initial_path = format!("/{root_id}/");
    queue.push_back((root_id.to_string(), 0usize, initial_path));

    let mut distinct_nodes = Vec::new();
    let mut seen_in_result = std::collections::HashSet::new();
    distinct_nodes.push(root_id.to_string());
    seen_in_result.insert(root_id.to_string());

    while let Some((node_id, depth, visited_path)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        let query_sql = match direction {
            "forward" => "SELECT dst_id FROM knowledge_edges WHERE src_id = ?1",
            "reverse" => "SELECT src_id FROM knowledge_edges WHERE dst_id = ?1",
            "undirected" => {
                "SELECT CASE WHEN src_id = ?1 THEN dst_id ELSE src_id END \
                 FROM knowledge_edges \
                 WHERE src_id = ?1 OR dst_id = ?1"
            }
            _ => continue,
        };

        let mut rows = conn.query(query_sql, params![node_id.as_str()]).await?;
        while let Some(row) = rows.next().await? {
            let next_id: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let needle = format!("/{next_id}/");
            if !visited_path.contains(&needle) {
                let next_path = format!("{visited_path}{next_id}/");
                if seen_in_result.insert(next_id.clone()) {
                    distinct_nodes.push(next_id.clone());
                }
                queue.push_back((next_id, depth + 1, next_path));
            }
        }
    }

    Ok(distinct_nodes)
}
