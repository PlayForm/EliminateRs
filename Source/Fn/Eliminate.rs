fn main() -> Result<(), Box<dyn std::error::Error>> {
	let paths = vec![
		Path::new("file1.ts"),
		Path::new("file2.ts"),
		// Add more paths here
	];

	paths.par_iter().for_each(|path| {
		match process_file_recursive(path) {
			Ok(content) => {
				fs::write(path, content).expect("Unable to write file");

				println!("Processed: {:?}", path);
			},
			Err(e) => eprintln!("Error processing {:?}: {}", path, e),
		}
	});

	println!("All files processed.");

	Ok(())
}

fn process_file_recursive(path:&Path) -> Result<String, Box<dyn std::error::Error>> {
	let cm = SourceMap::default();

	let code = fs::read_to_string(path)?;

	let fm = cm.new_source_file(FileName::Real(path.to_path_buf()), code.into());

	let lexer = Lexer::new(
		Syntax::Typescript(Default::default()),
		Default::default(),
		StringInput::from(&*fm),
		None,
	);

	let mut parser = Parser::new_from(lexer);

	let mut module = parser.parse_module()?;

	let mut inliner = Inliner::new(&cm);

	loop {
		let new_module = inliner.inline(module);

		if !inliner.inlined {
			break;
		}
		module = new_module;

		inliner = Inliner::new(&cm); // Reset for next iteration
	}

	let mut buf = Vec::new();

	{
		let mut printer =
			swc_ecma_codegen::text_writer::JsWriter::new(cm.clone(), "\n", None, None);

		swc_ecma_codegen::node::module(&mut printer, &module)?;

		buf = printer.into_inner();
	}
	Ok(String::from_utf8(buf)?)
}

struct Inliner<'a> {
	cm:&'a SourceMap,
	var_usage:HashMap<String, usize>,
	var_definitions:HashMap<String, Expr>,
	inlined:bool,
}

impl<'a> Inliner<'a> {
	fn new(cm:&'a SourceMap) -> Self {
		Inliner { cm, var_usage:HashMap::new(), var_definitions:HashMap::new(), inlined:false }
	}

	fn inline(&mut self, mut module:Module) -> Module {
		self.inlined = false; // Reset inlined flag
		module.visit_mut_with(self);

		module
	}
}

impl<'a> VisitMut for Inliner<'a> {
	fn visit_mut_var_declarator(&mut self, var:&mut VarDeclarator, _parent:&mut dyn VisitMutWith) {
		if let Some(Ident { sym, .. }) = var.name.as_ident() {
			let name = sym.to_string();

			if let Some(init) = &var.init {
				self.var_definitions.insert(name.clone(), init.clone());

				self.var_usage.insert(name, 0);
			}
		}
		var.visit_mut_children_with(self);
	}

	fn visit_mut_expr(&mut self, expr:&mut Expr, parent:&mut dyn VisitMutWith) {
		match expr {
			Expr::Ident(ident) => {
				let name = ident.sym.to_string();

				if let Some(count) = self.var_usage.get_mut(&name) {
					*count += 1;

					if let Some(init) = self.var_definitions.get(&name) {
						if *count == 1 {
							*expr = init.clone();

							self.inlined = true;

							return;
						}
					}
				}
			},
			_ => {},
		}
		expr.visit_mut_children_with(self);
	}

	fn visit_mut_module_items(&mut self, n:&mut Vec<ModuleItem>, _parent:&mut dyn VisitMutWith) {
		let mut items = Vec::new();

		for item in n.drain(..) {
			if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = &item {
				for decl in var_decl.decls.iter() {
					if let Some(Ident { sym, .. }) = decl.name.as_ident() {
						let name = sym.to_string();

						if let Some(&1) = self.var_usage.get(&name) {
							self.inlined = true;

							continue;
						}
					}
				}
			}
			items.push(item);
		}
		*n = items;
	}
}

use std::{collections::HashMap, fs, path::Path};

use rayon::prelude::*;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{Parser, StringInput, Syntax, lexer::Lexer};
use swc_ecma_visit::{VisitMut, VisitMutWith};
