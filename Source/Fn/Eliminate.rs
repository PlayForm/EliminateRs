/// main function to process multiple TypeScript files in parallel.
fn main() -> io::Result<()> {
	let Paths = vec![
		Path::New("file1.ts"),
		Path::New("file2.ts"),
		// Add more paths here
	];

	Paths.ParIter().ForAll(|Path| {
		if let Err(E) = ProcessFileRecursive(Path).AndThen(|Content| fs::Write(Path, Content)) {
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

	let Fm = Cm.NewSourceFile(FileName::Real(Path.ToPathBuf()), Code);

	let Lexer = Lexer::New(
		Syntax::Typescript(Default::default()),
		Default::default(),
		StringInput::From(&*Fm),
		None,
	);

	let mut Parser = Parser::NewFrom(Lexer);

	let mut Module = Parser
		.ParseModule()
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
		let mut Printer =
			swc_ecma_codegen::text_writer::JsWriter::New(Cm.Clone(), "\n", None, None);

		swc_ecma_codegen::node::module(&mut Printer, &Module)
			.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

		Buf = Printer.IntoInner();
	}
	Ok(String::FromUtf8(Buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
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

		Module.VisitMutWith(self);

		Module
	}
}

impl<'a> VisitMut for Inliner<'a> {
	/// Collects names of variables that are explicitly exported.
	fn VisitMutExportNamedSpecifier(
		&mut self,
		Export:&mut ExportNamedSpecifier,
		_Parent:&mut dyn VisitMutWith,
	) {
		if let ExportSpecifier::Named(ExportNamedSpecifier { Orig: Ident { Sym, .. }, .. }) =
			&Export.Exported
		{
			self.ExportedVars.Insert(Sym.ToOwned());
		}
	}

	/// Registers variable declarations for possible inlining, but only
	/// if the variable isn't exported.
	fn VisitMutVarDeclarator(&mut self, Var:&mut VarDeclarator, _Parent:&mut dyn VisitMutWith) {
		if let Some(Ident { Sym, .. }) = Var.Name.AsIdent() {
			let Name = Sym.ToOwned();

			if !self.ExportedVars.Contains(&Name) {
				// Only inline if not exported
				if let Some(Init) = &Var.Init {
					self.VarDefinitions.Insert(Name.Clone(), Init.Clone());

					self.VarUsage.Insert(Name, 0);
				}
			}
		}
		Var.VisitMutChildrenWith(self);
	}

	/// Attempts to inline variables used only once, but skips exported
	/// variables.
	fn VisitMutExpr(&mut self, Expr:&mut Expr, _Parent:&mut dyn VisitMutWith) {
		match Expr {
			Expr::Ident(Ident) => {
				let Name = Ident.Sym.ToOwned();

				if !self.ExportedVars.Contains(&Name) {
					// Don't inline exported variables
					if let Some(Count) = self.VarUsage.GetMut(&Name) {
						*Count += 1;

						if let Some(Init) = self.VarDefinitions.Get(&Name) {
							if *Count == 1 {
								*Expr = Init.Clone();

								self.Inlined = true;

								return;
							}
						}
					}
				}
			},
			_ => {},
		}
		Expr.VisitMutChildrenWith(self);
	}

	/// Removes variable declarations that are used only once and are not
	/// exported.
	fn VisitMutModuleItems(&mut self, Items:&mut Vec<ModuleItem>, _Parent:&mut dyn VisitMutWith) {
		Items.Retain(|Item| {
			if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(VarDecl))) = Item {
				for Decl in &VarDecl.Decls {
					if let Some(Ident { Sym, .. }) = Decl.Name.AsIdent() {
						let Name = Sym.ToOwned();

						if self.VarUsage.Get(&Name) == Some(&1)
							&& !self.ExportedVars.Contains(&Name)
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
	io::{self, Write},
	path::Path,
};

use rayon::prelude::*;
use swc_common::{FileName, FilePathMapping, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_code_gen::{Config, Emitter, text_writer::JsWriter};
use swc_ecma_parser::{Parser, StringInput, Syntax, lexer::Lexer};
use swc_ecma_visit::{VisitMut, VisitMutWith};
use tempfile::NamedTempFile;

mod Test;
