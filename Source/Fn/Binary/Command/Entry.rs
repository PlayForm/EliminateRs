/// Generates a list of file paths from the specified root directory, excluding
/// paths that match any of the specified exclude patterns.
///
/// # Arguments
///
/// * `Option` - A reference to an `Option` struct containing the following
///   fields:
///   - `Exclude`: A vector of strings representing patterns to exclude.
///   - `Pattern`: A string pattern to match against the last element of each
///     entry.
///   - `Root`: The root directory to start the walk from.
///   - `Separator`: The separator used for splitting file paths.
///
/// # Returns
///
/// Returns a vector of vectors, where each inner vector contains the components
/// of a file path split by the specified separator.
///
/// # Panics
///
/// This function will panic if it encounters an error while reading a directory
/// entry.
///
/// # Example
///
/// ```
/// let options = Option {
/// 	Exclude:vec!["node_modules".to_string(), "target".to_string()],
/// 	Pattern:".git".to_string(),
/// 	Root:".".to_string(),
/// 	Separator:'/',
/// };
/// let paths = Fn(&options);
/// for path in paths {
/// 	println!("{:?}", path);
/// }
/// ```
pub fn Fn(Option { Exclude, Pattern, Root, Separator, .. }:&Option) -> Return {
	WalkDir::new(Root)
		.follow_links(false)
		.into_iter()
		.filter_map(|Entry| {
			let Path = Entry.expect("Cannot Entry.").path().display().to_string();

			// TODO: Separate this into Entry/Exclude.rs
			if !Exclude
				.clone()
				.into_iter()
				.filter(|Exclude| *Pattern != *Exclude)
				.any(|Exclude| Path.contains(&Exclude))
			{
				Some(Path.split(*Separator).map(|Entry| Entry.to_string()).collect())
			} else {
				None
			}
		})
		.collect::<Vec<_>>()
}

use walkdir::WalkDir;

use crate::Struct::Binary::Command::{Entry::Type as Return, Option::Struct as Option};
