/// Creates and returns the command-line argument matches for the `Summary`
/// application.
///
/// This function sets up the command-line interface using the `clap` crate,
/// defining various arguments and their properties such as short and long
/// names, help messages, default values, and whether they are required.
///
/// # Returns
///
/// Returns an `ArgMatches` instance containing the parsed command-line
/// arguments.
///
/// # Arguments
///
/// * `Exclude` - An optional argument to specify patterns to exclude. Default
///   is "node_modules".
/// * `Omit` - An optional argument to specify patterns to omit. Default values
///   are:
///   - "(?i)documentation"
///   - "(?i)target"
///   - "(?i)changelog\.md$"
///   - "(?i)summary\.md$"
/// * `Parallel` - An optional flag to enable parallel processing.
/// * `Pattern` - An optional argument to specify a pattern to match. Default is
///   ".git".
/// * `Root` - An optional argument to specify the root directory. Default is
///   ".".
///
/// # Example
///
/// ```rust
/// let matches = Fn();
/// let exclude = matches.value_of("Exclude").unwrap_or("node_modules");
/// let omit = matches.values_of("Omit").unwrap_or_default().collect::<Vec<_>>();
/// let parallel = matches.is_present("Parallel");
/// let pattern = matches.value_of("Pattern").unwrap_or(".git");
/// let root = matches.value_of("Root").unwrap_or(".");
/// ```
///
/// # Errors
///
/// This function will panic if there are issues with the argument definitions
/// or parsing.
pub fn Fn() -> ArgMatches {
	Command::new("Summary")
		.version(env!("CARGO_PKG_VERSION"))
		.author("🖋️ Source — 👐🏻 Open — <Source/Open@PlayForm.LTD>")
		.about("🗣️ Summary —")
		.arg(
			Arg::new("Exclude")
				.short('E')
				.long("Exclude")
				.display_order(4)
				.value_name("EXCLUDE")
				.required(false)
				.help("🚫 Exclude —")
				.default_value("node_modules"),
		)
		.arg(
			Arg::new("Omit")
				.short('O')
				.long("Omit")
				.display_order(6)
				.value_name("OMIT")
				.required(false)
				.help("🚫 Omit —")
				.action(clap::ArgAction::Append)
				.default_values([
					"(?i)documentation",
					"(?i)target",
					r"(?i)changelog\.md$",
					r"(?i)summary\.md$",
				]),
		)
		.arg(
			Arg::new("Parallel")
				.short('P')
				.long("Parallel")
				.action(SetTrue)
				.display_order(2)
				.value_name("PARALLEL")
				.required(false)
				.help("⏩ Parallel —"),
		)
		.arg(
			Arg::new("Pattern")
				.long("Pattern")
				.display_order(5)
				.value_name("PATTERN")
				.required(false)
				.help("🔍 Pattern —")
				.default_value(".git"),
		)
		.arg(
			Arg::new("Root")
				.short('R')
				.long("Root")
				.display_order(3)
				.value_name("ROOT")
				.required(false)
				.help("📂 Root —")
				.default_value("."),
		)
		.get_matches()
}

use clap::{Arg, ArgAction::SetTrue, ArgMatches, Command};

pub mod Entry;
pub mod Parallel;
pub mod Sequential;
