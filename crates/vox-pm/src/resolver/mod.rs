//! Semantic versions and dependency resolution helpers.

mod error;
mod semver;
mod version_req;
mod resolve;

pub use error::ResolverError;
pub use resolve::{AvailablePackage, ResolvedDep, Resolver};
pub use semver::SemVer;
pub use version_req::VersionReq;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_semver() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert!(v.pre.is_none());
    }

    #[test]
    fn test_parse_semver_prerelease() {
        let v = SemVer::parse("1.0.0-beta.1").unwrap();
        assert_eq!(v.pre, Some("beta.1".to_string()));
    }

    #[test]
    fn test_parse_semver_short() {
        let v = SemVer::parse("2").unwrap();
        assert_eq!(
            v,
            SemVer {
                major: 2,
                minor: 0,
                patch: 0,
                pre: None
            }
        );
    }

    #[test]
    fn test_semver_ordering() {
        let v1 = SemVer::parse("1.0.0").unwrap();
        let v2 = SemVer::parse("1.0.1").unwrap();
        let v3 = SemVer::parse("1.1.0").unwrap();
        let v4 = SemVer::parse("2.0.0").unwrap();
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
    }

    #[test]
    fn test_prerelease_less_than_release() {
        let pre = SemVer::parse("1.0.0-alpha").unwrap();
        let rel = SemVer::parse("1.0.0").unwrap();
        assert!(pre < rel);
    }

    #[test]
    fn test_caret_req() {
        let req = VersionReq::parse("^1.2.0").unwrap();
        assert!(req.matches(&SemVer::parse("1.2.0").unwrap()));
        assert!(req.matches(&SemVer::parse("1.3.0").unwrap()));
        assert!(req.matches(&SemVer::parse("1.99.99").unwrap()));
        assert!(!req.matches(&SemVer::parse("2.0.0").unwrap()));
        assert!(!req.matches(&SemVer::parse("1.1.0").unwrap()));
    }

    #[test]
    fn test_caret_zero_major() {
        let req = VersionReq::parse("^0.2.0").unwrap();
        assert!(req.matches(&SemVer::parse("0.2.0").unwrap()));
        assert!(req.matches(&SemVer::parse("0.2.5").unwrap()));
        assert!(!req.matches(&SemVer::parse("0.3.0").unwrap()));
    }

    #[test]
    fn test_tilde_req() {
        let req = VersionReq::parse("~1.2.0").unwrap();
        assert!(req.matches(&SemVer::parse("1.2.0").unwrap()));
        assert!(req.matches(&SemVer::parse("1.2.9").unwrap()));
        assert!(!req.matches(&SemVer::parse("1.3.0").unwrap()));
    }

    #[test]
    fn test_compound_req() {
        let req = VersionReq::parse(">=1.0.0, <2.0.0").unwrap();
        assert!(req.matches(&SemVer::parse("1.0.0").unwrap()));
        assert!(req.matches(&SemVer::parse("1.5.0").unwrap()));
        assert!(!req.matches(&SemVer::parse("2.0.0").unwrap()));
        assert!(!req.matches(&SemVer::parse("0.9.0").unwrap()));
    }

    #[test]
    fn test_any_req() {
        let req = VersionReq::parse("*").unwrap();
        assert!(req.matches(&SemVer::parse("0.0.1").unwrap()));
        assert!(req.matches(&SemVer::parse("999.0.0").unwrap()));
    }

    #[test]
    fn test_resolver_basic() {
        let mut resolver = Resolver::new();
        resolver.add_available("foo", SemVer::parse("1.0.0").unwrap());
        resolver.add_available("foo", SemVer::parse("1.1.0").unwrap());
        resolver.add_available("foo", SemVer::parse("2.0.0").unwrap());

        let result = resolver
            .resolve(&[("foo".into(), "^1.0".into(), vec![])])
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "foo");
        assert_eq!(result[0].version, SemVer::parse("1.1.0").unwrap());
    }

    #[test]
    fn test_resolver_transitive() {
        let mut resolver = Resolver::new();
        resolver.add_available("app", SemVer::parse("1.0.0").unwrap());
        resolver.add_available("core", SemVer::parse("2.0.0").unwrap());
        resolver.add_available("core", SemVer::parse("2.1.0").unwrap());
        resolver.add_available("utils", SemVer::parse("0.5.0").unwrap());

        resolver.add_deps(
            "app",
            SemVer::parse("1.0.0").unwrap(),
            vec![("core".into(), "^2.0".into(), false, vec![])],
        );
        resolver.add_deps(
            "core",
            SemVer::parse("2.1.0").unwrap(),
            vec![("utils".into(), "^0.5".into(), false, vec![])],
        );

        let result = resolver
            .resolve(&[("app".into(), "^1.0".into(), vec![])])
            .unwrap();

        assert_eq!(result.len(), 3);
        let names: Vec<&str> = result.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"app"));
        assert!(names.contains(&"core"));
        assert!(names.contains(&"utils"));
    }

    #[test]
    fn test_resolver_not_found() {
        let resolver = Resolver::new();
        let result = resolver.resolve(&[("missing".into(), "^1.0".into(), vec![])]);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolver_no_matching_version() {
        let mut resolver = Resolver::new();
        resolver.add_available("foo", SemVer::parse("1.0.0").unwrap());

        let result = resolver.resolve(&[("foo".into(), "^2.0".into(), vec![])]);
        assert!(result.is_err());
    }
}
