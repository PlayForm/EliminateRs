/// main function to process multiple TypeScript files in parallel.
fn main() -> io::Result<()> {
	let Paths = vec![
		Path::new("file1.ts"),
		Path::new("file2.ts"),
		// Add more paths here
	];

	Paths.par_iter().for_each(|Path| {
		if let Err(E) = ProcessFileRecursive(Path).and_then(|Content| fs::write(Path, Content)) {
			eprintln!("Error processing {:?}: {}", Path, E);
		} else {
			println!("Processed: {:?}", Path);
		}
	});

	println!("All files processed.");

	Ok(())
}

/// Recursively processes a TypeScript file, inlining variables until no more
/// inlining is possible.
fn ProcessFileRecursive(Path:&Path) -> io::Result<String> {
	let Cm = SourceMap::default();

	let Code = fs::ReadToString(Path)?;

	let Fm = Cm.new_source_file(Rc::new(FileName::Real(Path.to_path_buf())), Code);

	let Lexer = Lexer::new(
		Syntax::Typescript(Default::default()),
		Default::default(),
		StringInput::from(&*Fm),
		None,
	);

	let mut Parser = Parser::new_from(Lexer);

	let mut Module = Parser
		.parse_module()
		.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

	let mut Inliner = Inliner::New(&Cm);

	loop {
		let NewModule = Inliner.Inline(Module);

		if !Inliner.Inlined {
			break;
		}

		Module = NewModule;

		Inliner = Inliner::New(&Cm); // Reset for next iteration
	}

	let mut Buf = Vec::new();

	{
		let mut Printer = JsWriter::new(Rc::new(Cm), "\n", None, None);

		swc_ecma_codegen::node::module(&mut Printer, &Module)
			.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

		Buf = Printer.IntoInner();
	}

	Ok(String::from_utf8(Buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
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

/// `Inliner` struct holds the state needed for inlining variables while
/// processing TypeScript code.
struct Inliner<'a> {
	/// The source map used for tracking source locations.
	Cm:&'a SourceMap,
	/// Counts how many times each variable is used.
	VarUsage:HashMap<String, usize>,
	/// Stores the initial value expressions for variables.
	VarDefinitions:HashMap<String, Expr>,
	/// Tracks which variables are exported and should not be inlined.
	ExportedVars:HashSet<String>,
	/// Flag to indicate if any inlining occurred during the last pass.
	Inlined:bool,
}

impl<'a> Inliner<'a> {
	/// Creates a new `Inliner` instance with the given `SourceMap`.
	fn New(Cm:&'a SourceMap) -> Self {
		Inliner {
			Cm,
			VarUsage:HashMap::new(),
			VarDefinitions:HashMap::new(),
			ExportedVars:HashSet::new(),
			Inlined:false,
		}
	}

	/// Performs a single pass of inlining on the given module,
	/// setting `Inlined` to true if any inlining occurs.
	fn Inline(&mut self, mut Module:Module) -> Module {
		self.Inlined = false;

		Module.visit_mut_with(self);

		Module
	}
}

impl<'a> VisitMut for Inliner<'a> {
	/// Collects names of variables that are explicitly exported.
	fn visit_mut_export_named_specifier(
		&mut self,
		Export:&mut ExportNamedSpecifier,
	) {
		if let ModuleExportName::Ident(Ident { sym, .. }) = &Export.orig {
			self.ExportedVars.insert(sym.to_string());
		}
	}

	/// Registers variable declarations for possible inlining, but only
	/// if the variable isn't exported.
	fn visit_mut_var_declarator(
		&mut self,
		Var:&mut VarDeclarator,
	) {
		if let Pat::Ident(BindingIdent { id, .. }) = Var.name {
			let Name:String = id.sym.to_string(); // Convert to String right away

			if !self.ExportedVars.contains(&Name) {
				// Only inline if not exported
				if let Some(Init) = &Var.init {
					self.VarDefinitions.insert(Name.clone(), (**Init).clone());

					self.VarUsage.insert(Name, 0);
				}
			}
		}

		Var.visit_mut_children_with(self);
	}

	/// Attempts to inline variables used only once, but skips exported
	/// variables.
	fn visit_mut_expr(&mut self, Expr:&mut Expr, _Parent:&mut dyn VisitMutWith) {
		match Expr {
			Expr::Ident(Ident { sym, .. }) => {
				let Name = sym.to_string();

				if !self.ExportedVars.contains(&Name) {
					// Don't inline exported variables
					if let Some(Count) = self.VarUsage.get_mut(&Name) {
						*Count += 1;

						if let Some(Init) = self.VarDefinitions.get(&Name) {
							if *Count == 1 {
								*Expr = Init.clone();

								self.Inlined = true;

								return;
							}
						}
					}
				}
			},
			_ => {},
		}

		Expr.visit_mut_children_with(self);
	}

	/// Removes variable declarations that are used only once and are not
	/// exported.
	fn visit_mut_module_items(
		&mut self,
		Items:&mut Vec<ModuleItem>
	) {
		Items.retain(|Item| {
			if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(VarDecl))) = Item {
				for Decl in &VarDecl.decls {
					if let Pat::Ident(BindingIdent { id: Name, .. }) = Decl.name {
						if self.VarUsage.get(&Name) == Some(&1)
							&& !self.ExportedVars.contains(&Name)
						{
							self.Inlined = true;

							return false; // Remove this declaration if not exported and used once
						}
					}
				}
			}

			true
		});
	}
}

use std::{
	collections::{HashMap, HashSet},
	fs,
	io::{self},
	path::Path,
	rc::Rc,
};

use rayon::prelude::*;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_codegen::{Config, Emitter, text_writer::JsWriter};
use swc_ecma_parser::{Parser, StringInput, Syntax, lexer::Lexer};
use swc_ecma_visit::{VisitMut, VisitMutWith};
use tempfile::NamedTempFile;

mod Test;
