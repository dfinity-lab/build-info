use build_info_common::CrateInfo;

/// Depth of dependencies to collect
///
/// A dependency depth that is too high may crash the build process
#[derive(Clone, Copy, Debug)]
pub enum DependencyDepth {
	/// Do not collect any dependencies
	None,
	/// Collect all dependencies
	///
	/// This may crash the build process if your dependencies are very deep
	Full,
	/// Collect dependencies to this depth
	///
	/// A too-high value may crash the build process
	Depth(usize),
}

impl DependencyDepth {
	/// Returns false if the dependency collection depth has been reached.
	pub fn do_collect(&self, depth: usize) -> bool {
		match self {
			DependencyDepth::None => false,
			DependencyDepth::Full => true,
			DependencyDepth::Depth(limit) => depth <= *limit,
		}
	}
}

impl crate::BuildScriptOptions {
	/// Enables and disables runtime dependency collection.
	///
	/// NOTE: In this fork, crate information is read from the `CARGO_*` environment variables cargo sets for build
	/// scripts rather than by invoking `cargo metadata`. This keeps the build script working inside build systems
	/// (e.g. Bazel) that do not expose a resolvable `Cargo.toml`/workspace, but it also means dependency collection is
	/// not supported: these setters are retained for API compatibility but have no effect.
	pub fn collect_runtime_dependencies(mut self, collect_dependencies: DependencyDepth) -> Self {
		self.collect_runtime_dependencies = collect_dependencies;
		self
	}

	/// Enables and disables build dependency collection.
	///
	/// See the note on [`collect_runtime_dependencies`](Self::collect_runtime_dependencies): dependency collection is
	/// not supported in this fork.
	pub fn collect_build_dependencies(mut self, collect_dependencies: DependencyDepth) -> Self {
		self.collect_build_dependencies = collect_dependencies;
		self
	}

	/// Enables and disables dev dependency collection.
	///
	/// See the note on [`collect_runtime_dependencies`](Self::collect_runtime_dependencies): dependency collection is
	/// not supported in this fork.
	pub fn collect_dev_dependencies(mut self, collect_dependencies: DependencyDepth) -> Self {
		self.collect_dev_dependencies = collect_dependencies;
		self
	}

	/// Enables and disables runtime, build and dev dependency collection.
	///
	/// See the note on [`collect_runtime_dependencies`](Self::collect_runtime_dependencies): dependency collection is
	/// not supported in this fork.
	pub fn collect_dependencies(mut self, collect_dependencies: DependencyDepth) -> Self {
		self.collect_runtime_dependencies = collect_dependencies;
		self.collect_build_dependencies = collect_dependencies;
		self.collect_dev_dependencies = collect_dependencies;
		self
	}
}

/// Reads the current crate's information from the `CARGO_*` environment variables that cargo sets for build scripts.
///
/// Unlike upstream, this does not shell out to `cargo metadata`, so it works in sandboxed build systems (e.g. Bazel)
/// where no resolvable `Cargo.toml`/workspace is available. As a consequence, `available_features` and `dependencies`
/// are not populated.
pub(crate) fn read_crate_info() -> CrateInfo {
	let mut enabled_features = vec![];
	for (key, _) in std::env::vars() {
		if let Some(feature) = key.strip_prefix("CARGO_FEATURE_") {
			enabled_features.push(feature.to_ascii_lowercase());
		}
	}

	CrateInfo {
		name: std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME"),
		version: std::env::var("CARGO_PKG_VERSION")
			.expect("CARGO_PKG_VERSION")
			.parse()
			.expect("CARGO_PKG_VERSION: parse"),
		authors: std::env::var("CARGO_PKG_AUTHORS").map_or_else(
			|_| Vec::new(),
			|authors| authors.split(':').map(|author| author.to_string()).collect(),
		),
		license: std::env::var("CARGO_PKG_LICENSE").ok(),
		enabled_features,
		available_features: Default::default(),
		dependencies: Default::default(),
	}
}
