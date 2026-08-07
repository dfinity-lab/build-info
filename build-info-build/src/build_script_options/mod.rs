use core::sync::atomic::{AtomicBool, Ordering};

use build_info_common::{OptimizationLevel, VersionedString};

pub use self::crate_info::DependencyDepth;
use super::BuildInfo;

mod compiler;
mod crate_info;
mod target;
mod version_control;

/// Type to store any (optional) options for the build script.
pub struct BuildScriptOptions {
	/// Stores if the build info has already been generated
	consumed: bool,

	/// Enable runtime dependency collection
	collect_runtime_dependencies: DependencyDepth,

	/// Enable build dependency collection
	collect_build_dependencies: DependencyDepth,

	/// Enable dev dependency collection
	collect_dev_dependencies: DependencyDepth,
}
static BUILD_SCRIPT_RAN: AtomicBool = AtomicBool::new(false);

impl BuildScriptOptions {
	/// WARNING: Should only be called once!
	fn drop_to_build_info(&mut self) -> BuildInfo {
		assert!(!self.consumed);
		self.consumed = true;

		let profile = std::env::var("PROFILE").unwrap_or_else(|_| "UNKNOWN".to_string());
		let optimization_level = match std::env::var("OPT_LEVEL")
			.expect("Expected environment variable `OPT_LEVEL` to be set by cargo")
			.as_str()
		{
			"0" => OptimizationLevel::O0,
			"1" => OptimizationLevel::O1,
			"2" => OptimizationLevel::O2,
			"3" => OptimizationLevel::O3,
			"s" => OptimizationLevel::Os,
			"z" => OptimizationLevel::Oz,
			level => panic!("Unknown optimization level {level:?}"),
		};

		let compiler = compiler::get_info();
		let target = target::get_info();
		let crate_info = crate_info::read_crate_info();
		let version_control = version_control::get_info();

		let build_info = BuildInfo {
			profile,
			optimization_level,
			crate_info,
			compiler,
			target,
			version_control,
		};

		let mut bytes = Vec::new();
		let mut compressed = zstd::Encoder::new(&mut bytes, 22).expect("Could not create ZSTD encoder");
		ciborium::into_writer(&build_info, &mut compressed).unwrap();
		compressed.finish().unwrap();

		let string = z85::encode(&bytes);
		let versioned = VersionedString::build_info_common_versioned(string);
		let serialized = serde_json::to_string(&versioned).unwrap();

		println!("cargo:rustc-env=BUILD_INFO={serialized}");

		// Whenever any `cargo:rerun-if-changed` key is set, the default set is cleared.
		// Since we will need to emit such keys to trigger rebuilds when the vcs repository changes state,
		// we also have to emit the customary triggers again, or we will only be rerun in that exact case.
		rebuild_if_project_changes();

		build_info
	}

	/// Consumes the `BuildScriptOptions` and returns a `BuildInfo` object. Use this function if you wish to inspect the
	/// generated build information in `build.rs`.
	pub fn build(mut self) -> BuildInfo {
		self.drop_to_build_info()
	}
}

impl From<BuildScriptOptions> for BuildInfo {
	fn from(opts: BuildScriptOptions) -> BuildInfo {
		opts.build()
	}
}

impl Default for BuildScriptOptions {
	fn default() -> Self {
		let build_script_ran = BUILD_SCRIPT_RAN.swap(true, Ordering::SeqCst);
		assert!(!build_script_ran, "The build script may only be run once.");

		Self {
			consumed: false,
			collect_runtime_dependencies: DependencyDepth::None,
			collect_build_dependencies: DependencyDepth::None,
			collect_dev_dependencies: DependencyDepth::None,
		}
	}
}

impl Drop for BuildScriptOptions {
	fn drop(&mut self) {
		if !self.consumed {
			let _build_info = self.drop_to_build_info();
		}
	}
}

/// Emits a `cargo:rerun-if-changed` line for each file in the target project.
///
/// Only `*.rs` files are included. Unlike upstream, `Cargo.toml` and the workspace `Cargo.lock` are intentionally NOT
/// emitted: in a large monorepo those trigger rebuilds of every build-info consumer on any manifest/lockfile change,
/// and under sandboxed build systems (e.g. Bazel) their absolute paths are not meaningful anyway.
fn rebuild_if_project_changes() {
	for source in glob::glob_with(
		"**/*.rs",
		glob::MatchOptions {
			case_sensitive: false,
			require_literal_separator: false,
			require_literal_leading_dot: false,
		},
	)
	.unwrap()
	.map(|source| source.unwrap())
	{
		println!("cargo:rerun-if-changed={}", source.to_str().unwrap());
	}
}
