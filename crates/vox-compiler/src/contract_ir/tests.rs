use super::*;
use crate::ast::span::Span;
use crate::hir::{HirType, HirTypeDef, HirVariant};

fn span() -> Span {
    Span { start: 0, end: 0 }
}

fn td(name: &str, fields: Vec<(&str, HirType)>) -> HirTypeDef {
    HirTypeDef {
        id: crate::hir::DefId(0),
        name: name.into(),
        variants: vec![],
        fields: fields.into_iter().map(|(n, t)| (n.into(), t)).collect(),
        is_pub: true,
        span: span(),
    }
}

fn sum(name: &str, variants: Vec<(&str, Vec<(&str, HirType)>)>) -> HirTypeDef {
    HirTypeDef {
        id: crate::hir::DefId(0),
        name: name.into(),
        variants: variants
            .into_iter()
            .map(|(vn, vfs)| HirVariant {
                name: vn.into(),
                fields: vfs.into_iter().map(|(n, t)| (n.into(), t)).collect(),
                span: span(),
            })
            .collect(),
        fields: vec![],
        is_pub: true,
        span: span(),
    }
}

#[test]
fn struct_projection_carries_field_names_and_types() {
    let t = td(
        "User",
        vec![
            ("id", HirType::Named("int".into())),
            ("name", HirType::Named("str".into())),
        ],
    );
    let c = project::type_def(&t);
    assert_eq!(c.name, "User");
    match c.kind {
        ContractTypeKind::Struct { fields } => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "id");
            assert!(matches!(fields[0].ty, WireType::Number));
            assert_eq!(fields[1].name, "name");
            assert!(matches!(fields[1].ty, WireType::String));
        }
        _ => panic!("expected struct"),
    }
}

#[test]
fn sum_projection_emits_tagged_variants() {
    let t = sum(
        "Status",
        vec![
            ("Active", vec![]),
            ("Banned", vec![("reason", HirType::Named("str".into()))]),
        ],
    );
    let c = project::type_def(&t);
    match c.kind {
        ContractTypeKind::Sum { variants } => {
            assert_eq!(variants[0].tag, "Active");
            assert!(variants[0].fields.is_empty());
            assert_eq!(variants[1].tag, "Banned");
            assert_eq!(variants[1].fields[0].name, "reason");
        }
        _ => panic!("expected sum"),
    }
}

#[test]
fn option_field_marks_optional_and_unwraps_inner() {
    let t = td(
        "Profile",
        vec![(
            "nickname",
            HirType::Generic("Option".into(), vec![HirType::Named("str".into())]),
        )],
    );
    let c = project::type_def(&t);
    let ContractTypeKind::Struct { fields } = c.kind else {
        panic!("expected struct");
    };
    assert!(fields[0].optional);
    assert!(matches!(fields[0].ty, WireType::String));
}

#[test]
fn decimal_widens_to_string_per_wire_format_v1() {
    assert!(matches!(
        project_type(&HirType::Decimal),
        WireType::DecimalString
    ));
    assert!(matches!(
        project_type(&HirType::Named("Decimal".into())),
        WireType::DecimalString
    ));
}

#[test]
fn bigint_and_128bit_widen_to_string() {
    assert!(matches!(
        project_type(&HirType::Named("bigint".into())),
        WireType::BigIntString
    ));
    assert!(matches!(
        project_type(&HirType::Named("i128".into())),
        WireType::BigIntString
    ));
}

#[test]
fn datetime_aliases_widen_to_string() {
    for name in ["Date", "DateTime", "Instant", "Timestamp"] {
        assert!(
            matches!(
                project_type(&HirType::Named(name.into())),
                WireType::DateTimeString
            ),
            "{name} should widen to DateTimeString"
        );
    }
}

#[test]
fn list_of_t_projects_as_array() {
    let ty = HirType::Generic("list".into(), vec![HirType::Named("int".into())]);
    let WireType::Array(inner) = project_type(&ty) else {
        panic!("expected array");
    };
    assert!(matches!(*inner, WireType::Number));
}

#[test]
fn user_named_type_projects_as_ref() {
    assert!(matches!(
        project_type(&HirType::Named("Project".into())),
        WireType::Ref(name) if name == "Project"
    ));
}
