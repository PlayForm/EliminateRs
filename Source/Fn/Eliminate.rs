fn main() {
	let ts_code = r#"
		let a = 5;
		
		let b = a + 1;
		
		let c = b * 2;
		
		console.log(c);
    "#;

	let result = inline_variables(ts_code);

	println!("{}", result);
}

fn inline_variables(code:&str) -> String {
	let mut lines = code.lines().map(|s| s.to_string()).collect::<Vec<String>>();

	let mut var_usage = HashMap::new();

	let mut var_definitions = HashMap::new();

	// First pass: Collect definitions and usages
	for (i, line) in lines.iter().enumerate() {
		if let Some(var_name) =
			line.strip_prefix("let ").and_then(|s| s.split('=').next().map(|s| s.trim()))
		{
			var_definitions.insert(var_name.to_string(), (i, line));

			var_usage.insert(var_name.to_string(), 0);
		} else {
			for var in var_usage.keys() {
				if line.contains(var) {
					*var_usage.get_mut(var).unwrap() += 1;
				}
			}
		}
	}

	// Second pass: Inline variables used only once
	for (var, &(line_num, _)) in var_definitions.iter() {
		if let Some(&1) = var_usage.get(var) {
			let value =
				var_definitions[var].1.split('=').nth(1).unwrap().trim().trim_end_matches(';');

			for line in lines.iter_mut().skip(line_num + 1) {
				*line = line.replace(var, value);
			}
			lines.remove(line_num);
		}
	}

	lines.join("\n")
}

use std::collections::HashMap;
