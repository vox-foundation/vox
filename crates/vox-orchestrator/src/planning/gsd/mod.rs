//! Native GSD (Get Shit Done) workflow engine for `.planning/` parsing,
//! milestone/phase/verification lifecycle management, Arca CAS sync, and
//! wave-to-`PlanNode` DAG scheduling.

pub mod lifecycle;
pub mod parser;
pub mod scheduler;

pub use lifecycle::{
    GsdHealthReport, complete_milestone, generate_or_load_phase_plan, health_check,
    init_new_milestone, load_workspace_status, prepare_execute_phase, record_discuss_phase,
    record_verify_work, sync_planning_dir_to_db,
};
pub use parser::{
    ParsedPlanningState, parse_phase_plan_waves, parse_planning_workspace, render_phase_plan_md,
    render_state_json,
};
pub use scheduler::waves_to_plan_nodes;
