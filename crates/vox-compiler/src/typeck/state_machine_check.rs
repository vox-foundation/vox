//! Structural type checking for `state_machine` declarations (TASK-4.1).
//!
//! Validates:
//! - No duplicate state names → error `E_SM_DUP_STATE`
//! - Terminal state with outgoing transitions → error `E_SM_TERMINAL_TRANSITION`
//! - Transition references undeclared state (from/to) → error `E_SM_UNKNOWN_STATE`
//! - Empty state machine (no states) → warning `W_SM_EMPTY`

use crate::hir::nodes::state_machine::{HirStateMachineDecl, HirTransitionSource};
use crate::typeck::diagnostics::{DiagnosticCategory, TypeckSeverity};
use crate::typeck::Diagnostic;
use std::collections::HashSet;

/// Run all state-machine structural checks. Returns a list of diagnostics.
pub fn check_state_machines(machines: &[HirStateMachineDecl]) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    for machine in machines {
        // W_SM_EMPTY: no states declared
        if machine.states.is_empty() {
            out.push(Diagnostic {
                severity: TypeckSeverity::Warning,
                message: format!("state_machine `{}` has no states", machine.name),
                span: machine.span,
                expected_type: None,
                found_type: None,
                context: Some("add at least one `state` declaration".to_string()),
                suggestions: vec!["state Initial".to_string()],
                category: DiagnosticCategory::Typecheck,
                code: Some("W_SM_EMPTY".to_string()),
                fixes: vec![],
                line_col: None,
                missing_cases: vec![],
                ast_node_kind: Some("StateMachineDecl".to_string()),
            });
        }

        // Build set of declared state names, checking for duplicates.
        let mut declared_states: HashSet<&str> = HashSet::new();
        let mut terminal_states: HashSet<&str> = HashSet::new();

        for state in &machine.states {
            if !declared_states.insert(state.name.as_str()) {
                out.push(Diagnostic {
                    severity: TypeckSeverity::Error,
                    message: format!(
                        "duplicate state name `{}` in state_machine `{}`",
                        state.name, machine.name
                    ),
                    span: state.span,
                    expected_type: None,
                    found_type: None,
                    context: Some(format!(
                        "state_machine `{}` already has a state named `{}`",
                        machine.name, state.name
                    )),
                    suggestions: vec![],
                    category: DiagnosticCategory::Typecheck,
                    code: Some("E_SM_DUP_STATE".to_string()),
                    fixes: vec![],
                    line_col: None,
                    missing_cases: vec![],
                    ast_node_kind: Some("SmStateDecl".to_string()),
                });
            }
            if state.terminal {
                terminal_states.insert(state.name.as_str());
            }
        }

        // Check transitions
        for transition in &machine.transitions {
            // E_SM_TERMINAL_TRANSITION: terminal state with outgoing transition
            if let HirTransitionSource::State(ref from_name) = transition.from {
                if terminal_states.contains(from_name.as_str()) {
                    out.push(Diagnostic {
                        severity: TypeckSeverity::Error,
                        message: format!(
                            "terminal state `{}` cannot have outgoing transitions (event `{}`)",
                            from_name, transition.event
                        ),
                        span: transition.span,
                        expected_type: None,
                        found_type: None,
                        context: Some(format!(
                            "`{}` is declared `terminal` so no transitions can leave it",
                            from_name
                        )),
                        suggestions: vec!["remove the `terminal` modifier or remove the transition".to_string()],
                        category: DiagnosticCategory::Typecheck,
                        code: Some("E_SM_TERMINAL_TRANSITION".to_string()),
                        fixes: vec![],
                        line_col: None,
                        missing_cases: vec![],
                        ast_node_kind: Some("SmTransitionDecl".to_string()),
                    });
                }

                // E_SM_UNKNOWN_STATE: from references undeclared state
                if !declared_states.contains(from_name.as_str()) {
                    out.push(Diagnostic {
                        severity: TypeckSeverity::Error,
                        message: format!(
                            "transition `on {}` references undeclared state `{}` in `from` clause",
                            transition.event, from_name
                        ),
                        span: transition.span,
                        expected_type: None,
                        found_type: None,
                        context: Some(format!(
                            "state `{}` is not declared in state_machine `{}`",
                            from_name, machine.name
                        )),
                        suggestions: vec![],
                        category: DiagnosticCategory::Typecheck,
                        code: Some("E_SM_UNKNOWN_STATE".to_string()),
                        fixes: vec![],
                        line_col: None,
                        missing_cases: vec![],
                        ast_node_kind: Some("SmTransitionDecl".to_string()),
                    });
                }
            }

            // E_SM_UNKNOWN_STATE: to references undeclared state
            if !declared_states.contains(transition.to.as_str()) {
                out.push(Diagnostic {
                    severity: TypeckSeverity::Error,
                    message: format!(
                        "transition `on {}` references undeclared target state `{}`",
                        transition.event, transition.to
                    ),
                    span: transition.span,
                    expected_type: None,
                    found_type: None,
                    context: Some(format!(
                        "state `{}` is not declared in state_machine `{}`",
                        transition.to, machine.name
                    )),
                    suggestions: vec![],
                    category: DiagnosticCategory::Typecheck,
                    code: Some("E_SM_UNKNOWN_STATE".to_string()),
                    fixes: vec![],
                    line_col: None,
                    missing_cases: vec![],
                    ast_node_kind: Some("SmTransitionDecl".to_string()),
                });
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::span::Span;
    use crate::hir::nodes::state_machine::{
        HirStateDecl, HirStateMachineDecl, HirTransitionDecl,
        HirTransitionSource,
    };

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    fn make_state(name: &str, terminal: bool) -> HirStateDecl {
        HirStateDecl {
            name: name.to_string(),
            fields: vec![],
            terminal,
            span: dummy_span(),
        }
    }

    fn make_transition(event: &str, from: HirTransitionSource, to: &str) -> HirTransitionDecl {
        HirTransitionDecl {
            event: event.to_string(),
            event_params: vec![],
            from,
            to: to.to_string(),
            span: dummy_span(),
        }
    }

    #[test]
    fn clean_machine_no_diagnostics() {
        let machines = vec![HirStateMachineDecl {
            name: "Traffic".to_string(),
            states: vec![
                make_state("Green", false),
                make_state("Red", false),
                make_state("Off", true),
            ],
            transitions: vec![
                make_transition("Switch", HirTransitionSource::State("Green".to_string()), "Red"),
                make_transition("Switch", HirTransitionSource::State("Red".to_string()), "Green"),
            ],
            span: dummy_span(),
        }];
        assert!(check_state_machines(&machines).is_empty());
    }

    #[test]
    fn duplicate_state_name_produces_error() {
        let machines = vec![HirStateMachineDecl {
            name: "Bad".to_string(),
            states: vec![make_state("Idle", false), make_state("Idle", false)],
            transitions: vec![],
            span: dummy_span(),
        }];
        let diags = check_state_machines(&machines);
        assert!(diags.iter().any(|d| d.code.as_deref() == Some("E_SM_DUP_STATE")));
    }

    #[test]
    fn terminal_state_with_outgoing_transition_produces_error() {
        let machines = vec![HirStateMachineDecl {
            name: "Bad".to_string(),
            states: vec![make_state("Done", true), make_state("Idle", false)],
            transitions: vec![make_transition(
                "Restart",
                HirTransitionSource::State("Done".to_string()),
                "Idle",
            )],
            span: dummy_span(),
        }];
        let diags = check_state_machines(&machines);
        assert!(diags
            .iter()
            .any(|d| d.code.as_deref() == Some("E_SM_TERMINAL_TRANSITION")));
    }

    #[test]
    fn transition_to_undeclared_state_produces_error() {
        let machines = vec![HirStateMachineDecl {
            name: "Bad".to_string(),
            states: vec![make_state("Idle", false)],
            transitions: vec![make_transition(
                "Go",
                HirTransitionSource::State("Idle".to_string()),
                "Ghost",
            )],
            span: dummy_span(),
        }];
        let diags = check_state_machines(&machines);
        assert!(diags
            .iter()
            .any(|d| d.code.as_deref() == Some("E_SM_UNKNOWN_STATE")));
    }

    #[test]
    fn transition_from_undeclared_state_produces_error() {
        // The `from` state "Ghost" is not declared — should produce E_SM_UNKNOWN_STATE.
        let machines = vec![HirStateMachineDecl {
            name: "Bad".to_string(),
            states: vec![make_state("Idle", false), make_state("Done", false)],
            transitions: vec![make_transition(
                "Phantom",
                HirTransitionSource::State("Ghost".to_string()),
                "Done",
            )],
            span: dummy_span(),
        }];
        let diags = check_state_machines(&machines);
        assert!(
            diags
                .iter()
                .any(|d| d.code.as_deref() == Some("E_SM_UNKNOWN_STATE")),
            "Expected E_SM_UNKNOWN_STATE for unknown `from` state, got: {diags:?}"
        );
        assert!(
            diags.iter().any(|d| d
                .message
                .contains("Ghost")),
            "Diagnostic should name the unknown state 'Ghost'"
        );
    }
}
