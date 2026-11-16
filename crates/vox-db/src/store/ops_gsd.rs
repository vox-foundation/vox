//! GSD (Get Shit Done) milestone, phase, requirement, and CAS-backed document operations.

use turso::params;
use vox_orchestrator_types::gsd::{
    GsdDocument, GsdDocumentKind, GsdMilestone, GsdMilestoneStatus, GsdPhase, GsdPhaseStatus,
    GsdProgressSnapshot, GsdRequirement,
};

use crate::VoxDb;
use crate::store::types::StoreError;

impl VoxDb {
    /// Insert or update a GSD milestone row.
    pub async fn upsert_gsd_milestone(&self, milestone: &GsdMilestone) -> Result<(), StoreError> {
        let m = milestone.clone();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO gsd_milestones (
                        milestone_id, title, status, start_phase, end_phase,
                        created_at_ms, updated_at_ms, completed_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                     ON CONFLICT(milestone_id) DO UPDATE SET
                        title = excluded.title,
                        status = excluded.status,
                        start_phase = excluded.start_phase,
                        end_phase = excluded.end_phase,
                        updated_at_ms = excluded.updated_at_ms,
                        completed_at_ms = COALESCE(excluded.completed_at_ms, gsd_milestones.completed_at_ms)",
                    params![
                        m.milestone_id.as_str(),
                        m.title.as_str(),
                        m.status.as_str(),
                        i64::from(m.start_phase),
                        i64::from(m.end_phase),
                        m.created_at_ms,
                        m.updated_at_ms,
                        m.completed_at_ms,
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// List GSD milestones ordered by most recently updated first.
    pub async fn list_gsd_milestones(&self) -> Result<Vec<GsdMilestone>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT milestone_id, title, status, start_phase, end_phase,
                        created_at_ms, updated_at_ms, completed_at_ms
                 FROM gsd_milestones ORDER BY updated_at_ms DESC, milestone_id DESC",
                params![],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(GsdMilestone {
                milestone_id: r.get(0)?,
                title: r.get(1)?,
                status: GsdMilestoneStatus::from_db_str(&r.get::<String>(2)?),
                start_phase: u32::try_from(r.get::<i64>(3)?.max(0)).unwrap_or(1),
                end_phase: u32::try_from(r.get::<i64>(4)?.max(0)).unwrap_or(1),
                created_at_ms: r.get(5)?,
                updated_at_ms: r.get(6)?,
                completed_at_ms: r.get(7)?,
            });
        }
        Ok(out)
    }

    /// Load a single milestone by ID, if present.
    pub async fn get_gsd_milestone(
        &self,
        milestone_id: &str,
    ) -> Result<Option<GsdMilestone>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT milestone_id, title, status, start_phase, end_phase,
                        created_at_ms, updated_at_ms, completed_at_ms
                 FROM gsd_milestones WHERE milestone_id = ?1 LIMIT 1",
                params![milestone_id],
            )
            .await?;
        let Some(r) = rows.next().await? else {
            return Ok(None);
        };
        Ok(Some(GsdMilestone {
            milestone_id: r.get(0)?,
            title: r.get(1)?,
            status: GsdMilestoneStatus::from_db_str(&r.get::<String>(2)?),
            start_phase: u32::try_from(r.get::<i64>(3)?.max(0)).unwrap_or(1),
            end_phase: u32::try_from(r.get::<i64>(4)?.max(0)).unwrap_or(1),
            created_at_ms: r.get(5)?,
            updated_at_ms: r.get(6)?,
            completed_at_ms: r.get(7)?,
        }))
    }

    /// Insert or update a phase record under a milestone.
    pub async fn upsert_gsd_phase(&self, phase: &GsdPhase) -> Result<(), StoreError> {
        let p = phase.clone();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO gsd_phases (
                        milestone_id, phase_number, title, status, wave_count,
                        plan_session_id, updated_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(milestone_id, phase_number) DO UPDATE SET
                        title = excluded.title,
                        status = excluded.status,
                        wave_count = excluded.wave_count,
                        plan_session_id = COALESCE(excluded.plan_session_id, gsd_phases.plan_session_id),
                        updated_at_ms = excluded.updated_at_ms",
                    params![
                        p.milestone_id.as_str(),
                        i64::from(p.phase_number),
                        p.title.as_str(),
                        p.status.as_str(),
                        i64::from(p.wave_count),
                        p.plan_session_id.as_deref(),
                        p.updated_at_ms,
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// List phases for a milestone (or all phases if `milestone_id` is `None`) in ascending phase order.
    pub async fn list_gsd_phases(
        &self,
        milestone_id: Option<&str>,
    ) -> Result<Vec<GsdPhase>, StoreError> {
        let mut rows = if let Some(ms) = milestone_id {
            self.conn
                .query(
                    "SELECT milestone_id, phase_number, title, status, wave_count, plan_session_id, updated_at_ms
                     FROM gsd_phases WHERE milestone_id = ?1 ORDER BY phase_number ASC",
                    params![ms],
                )
                .await?
        } else {
            self.conn
                .query(
                    "SELECT milestone_id, phase_number, title, status, wave_count, plan_session_id, updated_at_ms
                     FROM gsd_phases ORDER BY milestone_id DESC, phase_number ASC",
                    params![],
                )
                .await?
        };
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(GsdPhase {
                milestone_id: r.get(0)?,
                phase_number: u32::try_from(r.get::<i64>(1)?.max(0)).unwrap_or(0),
                title: r.get(2)?,
                status: GsdPhaseStatus::from_db_str(&r.get::<String>(3)?),
                wave_count: u32::try_from(r.get::<i64>(4)?.max(0)).unwrap_or(0),
                plan_session_id: r.get(5)?,
                updated_at_ms: r.get(6)?,
            });
        }
        Ok(out)
    }

    /// Insert or update a traceable requirement item.
    pub async fn upsert_gsd_requirement(&self, req: &GsdRequirement) -> Result<(), StoreError> {
        let r = req.clone();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO gsd_requirements (
                        req_id, milestone_id, phase_number, description,
                        satisfied, verification_ref, updated_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(req_id) DO UPDATE SET
                        milestone_id = excluded.milestone_id,
                        phase_number = excluded.phase_number,
                        description = excluded.description,
                        satisfied = excluded.satisfied,
                        verification_ref = COALESCE(excluded.verification_ref, gsd_requirements.verification_ref),
                        updated_at_ms = excluded.updated_at_ms",
                    params![
                        r.req_id.as_str(),
                        r.milestone_id.as_str(),
                        i64::from(r.phase_number),
                        r.description.as_str(),
                        if r.satisfied { 1_i64 } else { 0_i64 },
                        r.verification_ref.as_deref(),
                        r.updated_at_ms,
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// List requirements for a milestone (or all requirements when `None`).
    pub async fn list_gsd_requirements(
        &self,
        milestone_id: Option<&str>,
    ) -> Result<Vec<GsdRequirement>, StoreError> {
        let mut rows = if let Some(ms) = milestone_id {
            self.conn
                .query(
                    "SELECT req_id, milestone_id, phase_number, description, satisfied, verification_ref, updated_at_ms
                     FROM gsd_requirements WHERE milestone_id = ?1 ORDER BY phase_number ASC, req_id ASC",
                    params![ms],
                )
                .await?
        } else {
            self.conn
                .query(
                    "SELECT req_id, milestone_id, phase_number, description, satisfied, verification_ref, updated_at_ms
                     FROM gsd_requirements ORDER BY milestone_id DESC, phase_number ASC, req_id ASC",
                    params![],
                )
                .await?
        };
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(GsdRequirement {
                req_id: r.get(0)?,
                milestone_id: r.get(1)?,
                phase_number: u32::try_from(r.get::<i64>(2)?.max(0)).unwrap_or(0),
                description: r.get(3)?,
                satisfied: r.get::<i64>(4)? != 0,
                verification_ref: r.get(5)?,
                updated_at_ms: r.get(6)?,
            });
        }
        Ok(out)
    }

    /// Store GSD markdown/JSON document content into Arca CAS (`objects`) and upsert its `gsd_documents` index row.
    pub async fn store_gsd_document(
        &self,
        doc_id: &str,
        milestone_id: &str,
        phase_number: Option<u32>,
        kind: GsdDocumentKind,
        rel_path: &str,
        content: &str,
        updated_at_ms: i64,
    ) -> Result<GsdDocument, StoreError> {
        let bytes = content.as_bytes();
        let cas_hash = self.store("gsd_doc", bytes).await?;
        let byte_len = u64::try_from(bytes.len()).unwrap_or(0);
        let doc = GsdDocument {
            doc_id: doc_id.to_string(),
            milestone_id: milestone_id.to_string(),
            phase_number,
            kind,
            rel_path: rel_path.to_string(),
            cas_hash,
            byte_len,
            updated_at_ms,
        };
        let d = doc.clone();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO gsd_documents (
                        doc_id, milestone_id, phase_number, kind, rel_path, cas_hash, byte_len, updated_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                     ON CONFLICT(doc_id) DO UPDATE SET
                        milestone_id = excluded.milestone_id,
                        phase_number = excluded.phase_number,
                        kind = excluded.kind,
                        rel_path = excluded.rel_path,
                        cas_hash = excluded.cas_hash,
                        byte_len = excluded.byte_len,
                        updated_at_ms = excluded.updated_at_ms",
                    params![
                        d.doc_id.as_str(),
                        d.milestone_id.as_str(),
                        d.phase_number.map(i64::from),
                        d.kind.as_str(),
                        d.rel_path.as_str(),
                        d.cas_hash.as_str(),
                        i64::try_from(d.byte_len).unwrap_or(0),
                        d.updated_at_ms,
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await?;
        Ok(doc)
    }

    /// List GSD document metadata rows optionally filtered by milestone and phase.
    pub async fn list_gsd_documents(
        &self,
        milestone_id: Option<&str>,
        phase_number: Option<u32>,
    ) -> Result<Vec<GsdDocument>, StoreError> {
        let mut rows = match (milestone_id, phase_number) {
            (Some(ms), Some(ph)) => {
                self.conn
                    .query(
                        "SELECT doc_id, milestone_id, phase_number, kind, rel_path, cas_hash, byte_len, updated_at_ms
                         FROM gsd_documents WHERE milestone_id = ?1 AND phase_number = ?2
                         ORDER BY updated_at_ms DESC, rel_path ASC",
                        params![ms, i64::from(ph)],
                    )
                    .await?
            }
            (Some(ms), None) => {
                self.conn
                    .query(
                        "SELECT doc_id, milestone_id, phase_number, kind, rel_path, cas_hash, byte_len, updated_at_ms
                         FROM gsd_documents WHERE milestone_id = ?1
                         ORDER BY updated_at_ms DESC, rel_path ASC",
                        params![ms],
                    )
                    .await?
            }
            (None, Some(ph)) => {
                self.conn
                    .query(
                        "SELECT doc_id, milestone_id, phase_number, kind, rel_path, cas_hash, byte_len, updated_at_ms
                         FROM gsd_documents WHERE phase_number = ?1
                         ORDER BY updated_at_ms DESC, rel_path ASC",
                        params![i64::from(ph)],
                    )
                    .await?
            }
            (None, None) => {
                self.conn
                    .query(
                        "SELECT doc_id, milestone_id, phase_number, kind, rel_path, cas_hash, byte_len, updated_at_ms
                         FROM gsd_documents ORDER BY updated_at_ms DESC, rel_path ASC",
                        params![],
                    )
                    .await?
            }
        };
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            let ph_opt: Option<i64> = r.get(2)?;
            out.push(GsdDocument {
                doc_id: r.get(0)?,
                milestone_id: r.get(1)?,
                phase_number: ph_opt.and_then(|v| u32::try_from(v).ok()),
                kind: GsdDocumentKind::from_db_str(&r.get::<String>(3)?),
                rel_path: r.get(4)?,
                cas_hash: r.get(5)?,
                byte_len: u64::try_from(r.get::<i64>(6)?.max(0)).unwrap_or(0),
                updated_at_ms: r.get(7)?,
            });
        }
        Ok(out)
    }

    /// Fetch a GSD document metadata header plus its UTF-8 content from Arca CAS.
    pub async fn get_gsd_document_content(
        &self,
        doc_id_or_path: &str,
    ) -> Result<Option<(GsdDocument, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT doc_id, milestone_id, phase_number, kind, rel_path, cas_hash, byte_len, updated_at_ms
                 FROM gsd_documents WHERE doc_id = ?1 OR rel_path = ?1
                 ORDER BY updated_at_ms DESC LIMIT 1",
                params![doc_id_or_path],
            )
            .await?;
        let Some(r) = rows.next().await? else {
            return Ok(None);
        };
        let ph_opt: Option<i64> = r.get(2)?;
        let doc = GsdDocument {
            doc_id: r.get(0)?,
            milestone_id: r.get(1)?,
            phase_number: ph_opt.and_then(|v| u32::try_from(v).ok()),
            kind: GsdDocumentKind::from_db_str(&r.get::<String>(3)?),
            rel_path: r.get(4)?,
            cas_hash: r.get(5)?,
            byte_len: u64::try_from(r.get::<i64>(6)?.max(0)).unwrap_or(0),
            updated_at_ms: r.get(7)?,
        };
        let bytes = self.get(&doc.cas_hash).await?;
        let text = String::from_utf8(bytes)
            .map_err(|e| StoreError::Db(format!("gsd_doc {} not valid UTF-8: {e}", doc.doc_id)))?;
        Ok(Some((doc, text)))
    }

    /// Build a full GSD progress snapshot for the specified milestone or latest active milestone.
    pub async fn gsd_progress_snapshot(
        &self,
        milestone_id: Option<&str>,
    ) -> Result<GsdProgressSnapshot, StoreError> {
        let milestones = self.list_gsd_milestones().await?;
        let active_milestone = if let Some(id) = milestone_id {
            milestones.into_iter().find(|m| m.milestone_id == id)
        } else {
            milestones
                .iter()
                .find(|m| m.status == GsdMilestoneStatus::Active)
                .cloned()
                .or_else(|| milestones.into_iter().next())
        };
        let target_id = active_milestone.as_ref().map(|m| m.milestone_id.as_str());
        let phases = self.list_gsd_phases(target_id).await?;
        let requirements = self.list_gsd_requirements(target_id).await?;
        let documents = self.list_gsd_documents(target_id, None).await?;
        let next_recommended_skill = recommend_next_gsd_skill(active_milestone.as_ref(), &phases);
        Ok(GsdProgressSnapshot {
            active_milestone,
            phases,
            requirements,
            documents,
            next_recommended_skill,
        })
    }
}

fn recommend_next_gsd_skill(milestone: Option<&GsdMilestone>, phases: &[GsdPhase]) -> String {
    let Some(ms) = milestone else {
        return "gsd-new-milestone".to_string();
    };
    if ms.status != GsdMilestoneStatus::Active {
        return "gsd-new-milestone".to_string();
    }
    if let Some(next) = phases
        .iter()
        .find(|p| p.status != GsdPhaseStatus::Completed)
    {
        match next.status {
            GsdPhaseStatus::Pending => "gsd-discuss-phase",
            GsdPhaseStatus::Discussing => "gsd-plan-phase",
            GsdPhaseStatus::Planned => "gsd-execute-phase",
            GsdPhaseStatus::Executing => "gsd-execute-phase",
            GsdPhaseStatus::Verifying => "gsd-verify-work",
            GsdPhaseStatus::Blocked => "gsd-debug",
            GsdPhaseStatus::Completed => "gsd-progress",
        }
        .to_string()
    } else if phases.is_empty() {
        "gsd-plan-phase".to_string()
    } else {
        "gsd-complete-milestone".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DbConfig;

    #[tokio::test]
    async fn ops_gsd_cas_roundtrip_works() {
        let db = VoxDb::connect(DbConfig::Memory)
            .await
            .expect("in-memory VoxDb");
        let ms = GsdMilestone {
            milestone_id: "v7.0".to_string(),
            title: "Deep GSD Integration".to_string(),
            status: GsdMilestoneStatus::Active,
            start_phase: 407,
            end_phase: 412,
            created_at_ms: 100,
            updated_at_ms: 100,
            completed_at_ms: None,
        };
        db.upsert_gsd_milestone(&ms).await.expect("upsert ms");
        db.upsert_gsd_phase(&GsdPhase {
            milestone_id: "v7.0".to_string(),
            phase_number: 407,
            title: "Base Skills".to_string(),
            status: GsdPhaseStatus::Planned,
            wave_count: 2,
            plan_session_id: Some("plan-407".to_string()),
            updated_at_ms: 110,
        })
        .await
        .expect("upsert phase");
        db.upsert_gsd_requirement(&GsdRequirement {
            req_id: "GSD-SKILL-01".to_string(),
            milestone_id: "v7.0".to_string(),
            phase_number: 407,
            description: "Ship 30 MIT GSD skills".to_string(),
            satisfied: true,
            verification_ref: Some(".planning/phases/407/VERIFICATION.md".to_string()),
            updated_at_ms: 120,
        })
        .await
        .expect("upsert req");

        let body = "# Phase 407 Plan\n\nWave 1: Ship skills.\n";
        let stored = db
            .store_gsd_document(
                "v7.0:407:plan",
                "v7.0",
                Some(407),
                GsdDocumentKind::Plan,
                ".planning/phases/407/407-01-PLAN.md",
                body,
                130,
            )
            .await
            .expect("store cas doc");
        assert_eq!(stored.byte_len, body.len() as u64);

        let (fetched, content) = db
            .get_gsd_document_content(".planning/phases/407/407-01-PLAN.md")
            .await
            .expect("query")
            .expect("present");
        assert_eq!(fetched.cas_hash, stored.cas_hash);
        assert_eq!(content, body);

        let snap = db
            .gsd_progress_snapshot(Some("v7.0"))
            .await
            .expect("snapshot");
        assert_eq!(snap.active_milestone.as_ref().unwrap().milestone_id, "v7.0");
        assert_eq!(snap.phases.len(), 1);
        assert_eq!(snap.requirements.len(), 1);
        assert_eq!(snap.documents.len(), 1);
        assert_eq!(snap.next_recommended_skill, "gsd-execute-phase");
    }
}
