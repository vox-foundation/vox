//! Dependency graph and resolution.

use std::collections::{BTreeMap, HashMap, VecDeque};

use super::error::ResolverError;
use super::semver::SemVer;
use super::version_req::VersionReq;

/// Type alias for the dependency graph map: (package, version) → [(dep_name, version_req, optional, features)]
type DepGraph = HashMap<(String, SemVer), Vec<(String, String, bool, Vec<String>)>>;

/// A resolved dependency with its exact version.
#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub name: String,
    pub version: SemVer,
    pub hash: String,
    pub features: Vec<String>,
}

/// Package metadata for the registry/available packages.
#[derive(Debug, Clone)]
pub struct AvailablePackage {
    pub name: String,
    pub versions: Vec<SemVer>,
    pub deps: BTreeMap<String, (SemVer, Vec<(String, String)>)>, // version -> [(dep_name, version_req)]
}

/// The dependency resolver.
pub struct Resolver {
    /// Available packages in the registry (name -> available versions).
    available: HashMap<String, Vec<SemVer>>,
    /// Dependencies for each package@version: (dep_name, version_req_string, optional, features).
    dep_graph: DepGraph,
    /// Feature map: (package, version) -> { feature_name: [implied_features_or_deps] }
    feature_graph: HashMap<(String, SemVer), std::collections::BTreeMap<String, Vec<String>>>,
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
            dep_graph: HashMap::new(),
            feature_graph: HashMap::new(),
        }
    }

    /// Register an available package version.
    pub fn add_available(&mut self, name: &str, version: SemVer) {
        self.available
            .entry(name.to_string())
            .or_default()
            .push(version);
    }

    /// Register dependencies for a specific package@version.
    pub fn add_deps(
        &mut self,
        name: &str,
        version: SemVer,
        deps: Vec<(String, String, bool, Vec<String>)>,
    ) {
        self.dep_graph.insert((name.to_string(), version), deps);
    }

    /// Register features for a specific package@version.
    pub fn add_features(
        &mut self,
        name: &str,
        version: SemVer,
        features: std::collections::BTreeMap<String, Vec<String>>,
    ) {
        self.feature_graph
            .insert((name.to_string(), version), features);
    }

    /// Resolve dependencies for a root set of requirements.
    /// Returns a flat list of resolved packages or an error.
    pub fn resolve(
        &self,
        root_deps: &[(String, String, Vec<String>)],
    ) -> Result<Vec<ResolvedDep>, ResolverError> {
        let mut resolved: BTreeMap<String, SemVer> = BTreeMap::new();
        // Packge -> activated features
        let mut active_features: HashMap<String, std::collections::HashSet<String>> =
            HashMap::new();

        let mut queue: VecDeque<(String, String, Vec<String>)> = root_deps
            .iter()
            .map(|(n, v, f)| (n.clone(), v.clone(), f.clone()))
            .collect();

        while let Some((name, version_req_str, requested_features)) = queue.pop_front() {
            let req = VersionReq::parse(&version_req_str)?;

            let is_new = !resolved.contains_key(&name);
            let selected = if let Some(existing) = resolved.get(&name) {
                if !req.matches(existing) {
                    return Err(ResolverError::Conflict(
                        name.clone(),
                        existing.to_string(),
                        version_req_str.clone(),
                    ));
                }
                existing.clone()
            } else {
                let versions = self
                    .available
                    .get(&name)
                    .ok_or_else(|| ResolverError::PackageNotFound(name.clone()))?;

                let mut candidates: Vec<&SemVer> =
                    versions.iter().filter(|v| req.matches(v)).collect();
                candidates.sort();
                candidates.reverse(); // highest first

                let selected = candidates.first().ok_or_else(|| {
                    ResolverError::NoMatchingVersion(name.clone(), version_req_str.clone())
                })?;

                resolved.insert(name.clone(), (*selected).clone());
                (*selected).clone()
            };

            let key = (name.clone(), selected.clone());
            let pkg_features = self.feature_graph.get(&key);

            let mut newly_activated = Vec::new();
            if is_new {
                let active = active_features.entry(name.clone()).or_default();
                if active.insert("default".to_string()) {
                    newly_activated.push("default".to_string());
                }
            }

            for f in requested_features {
                let active = active_features.entry(name.clone()).or_default();
                if active.insert(f.clone()) {
                    newly_activated.push(f);
                }
            }

            let mut final_new_features = Vec::new();
            let mut feature_queue = newly_activated.clone();
            while let Some(feat) = feature_queue.pop() {
                final_new_features.push(feat.clone());
                if let Some(feature_map) = pkg_features {
                    if let Some(implied) = feature_map.get(&feat) {
                        for imp in implied {
                            let active = active_features.entry(name.clone()).or_default();
                            if active.insert(imp.clone()) {
                                feature_queue.push(imp.clone());
                            }
                        }
                    }
                }
            }

            if is_new || !final_new_features.is_empty() {
                if let Some(deps) = self.dep_graph.get(&key) {
                    for (dep_name, dep_req, optional, dep_feat) in deps {
                        let mut should_add = !*optional;
                        if *optional {
                            let active = active_features.entry(name.clone()).or_default();
                            if active.contains(dep_name)
                                || active.contains(&format!("dep:{}", dep_name))
                            {
                                should_add = true;
                            }
                        }

                        if should_add {
                            let dep_active = active_features.entry(dep_name.clone()).or_default();
                            let mut has_new_dep_features = false;
                            for df in dep_feat {
                                if !dep_active.contains(df) {
                                    has_new_dep_features = true;
                                }
                            }
                            let dep_is_new = !resolved.contains_key(dep_name);

                            if dep_is_new || has_new_dep_features {
                                queue.push_back((
                                    dep_name.clone(),
                                    dep_req.clone(),
                                    dep_feat.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(resolved
            .into_iter()
            .map(|(name, version)| {
                let mut features: Vec<String> = active_features
                    .get(&name)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                features.sort();
                ResolvedDep {
                    name,
                    version,
                    hash: String::new(),
                    features,
                }
            })
            .collect())
    }
}
