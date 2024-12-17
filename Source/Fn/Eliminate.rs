fn main() -> Result<(), Box<dyn std::error::Error>> {
	let Paths = vec![
		Path::new("file1.ts"),
		Path::new("file2.ts"),
		// Add more paths here
	];

	Paths.par_iter().for_each(|Path| {
		match ProcessFileRecursive(Path) {
			Ok(Content) => {
				fs::write(Path, Content).expect("Unable to write file");

				println!("Processed: {:?}", Path);
			},
			Err(E) => eprintln!("Error processing {:?}: {}", Path, E),
		}
	});

	println!("All files processed.");

	Ok(())
}

fn ProcessFileRecursive(Path:&Path) -> Result<String, Box<dyn std::error::Error>> {
	let Cm = SourceMap::default();

	let Code = fs::read_to_string(Path)?;

	let Fm = Cm.new_source_file(FileName::Real(Path.to_path_buf()), Code.into());

	let Lexer = Lexer::new(
		Syntax::Typescript(Default::default()),
		Default::default(),
		StringInput::from(&*Fm),
		None,
	);

	let mut Parser = Parser::new_from(Lexer);

	let mut Module = Parser.parse_module()?;

	let mut Inliner = Inliner::New(&Cm);

	loop {
		let New_Module = Inliner.Inline(Module);

		if !Inliner.Inlined {
			break;
		}
		Module = New_Module;

		Inliner = Inliner::New(&Cm); // Reset for next iteration
	}

	let mut Buf = Vec::new();

	{
		let mut Printer =
			swc_ecma_codegen::text_writer::JsWriter::new(Cm.clone(), "\n", None, None);

		swc_ecma_codegen::node::module(&mut Printer, &Module)?;

		Buf = Printer.into_inner();
	}
	Ok(String::from_utf8(Buf)?)
}

// Helper function to convert to title case
fn to_title_case(s:&str) -> String {
	s.chars()
		.enumerate()
		.map(|(i, c)| {
			if i == 0 {
				c.to_uppercase().next().unwrap()
			} else {
				c.to_lowercase().next().unwrap()
			}
		})
		.collect()
}

struct Inliner<'a> {
	Cm:&'a SourceMap,
	Var_Usage:HashMap<String, usize>,
	Var_Definitions:HashMap<String, Expr>,
	Inlined:bool,
}

impl<'a> Inliner<'a> {
	fn New(Cm:&'a SourceMap) -> Self {
		Inliner { Cm, Var_Usage:HashMap::new(), Var_Definitions:HashMap::new(), Inlined:false }
	}

	fn Inline(&mut self, mut Module:Module) -> Module {
		self.Inlined = false; // Reset Inlined flag
		Module.visit_mut_with(self);

		Module
	}
}

impl<'a> VisitMut for Inliner<'a> {
	fn visit_mut_var_declarator(&mut self, Var:&mut VarDeclarator, _Parent:&mut dyn VisitMutWith) {
		if let Some(Ident { Sym, .. }) = Var.name.as_ident() {
			let Name = Sym.to_string();

			if let Some(Init) = &Var.init {
				self.Var_Definitions.insert(Name.clone(), Init.clone());

				self.Var_Usage.insert(Name, 0);
			}
		}
		Var.visit_mut_children_with(self);
	}

	fn visit_mut_expr(&mut self, Expr:&mut Expr, Parent:&mut dyn VisitMutWith) {
		match Expr {
			Expr::Ident(Ident) => {
				let Name = Ident.sym.to_string();

				if let Some(Count) = self.Var_Usage.get_mut(&Name) {
					*Count += 1;

					if let Some(Init) = self.Var_Definitions.get(&Name) {
						if *Count == 1 {
							*Expr = Init.clone();

							self.Inlined = true;

							return;
						}
					}
				}
			},
			_ => {},
		}
		Expr.visit_mut_children_with(self);
	}

	fn visit_mut_module_items(&mut self, N:&mut Vec<ModuleItem>, _Parent:&mut dyn VisitMutWith) {
		let mut Items = Vec::new();

		for Item in N.drain(..) {
			if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(Var_Decl))) = &Item {
				for Decl in Var_Decl.decls.iter() {
					if let Some(Ident { Sym, .. }) = Decl.name.as_ident() {
						let Name = Sym.to_string();

						if let Some(&1) = self.Var_Usage.get(&Name) {
							self.Inlined = true;

							continue;
						}
					}
				}
			}
			Items.push(Item);
		}
		*N = Items;
	}
}

use std::{collections::HashMap, fs, path::Path};

use rayon::prelude::*;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{Parser, StringInput, Syntax, lexer::Lexer};
use swc_ecma_visit::{VisitMut, VisitMutWith};
