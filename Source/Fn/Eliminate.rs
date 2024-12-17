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

struct Inliner<'a> {
	Cm:&'a SourceMap,
	VarUsage:HashMap<String, usize>,
	VarDefinitions:HashMap<String, Expr>,
	Inlined:bool,
}

impl<'a> Inliner<'a> {
	fn New(Cm:&'a SourceMap) -> Self {
		Inliner { Cm, VarUsage:HashMap::new(), VarDefinitions:HashMap::new(), Inlined:false }
	}

	fn Inline(&mut self, mut Module:Module) -> Module {
		self.Inlined = false;

		Module.VisitMutWith(self);

		Module
	}
}

impl<'a> VisitMut for Inliner<'a> {
	fn VisitMutVarDeclarator(&mut self, Var:&mut VarDeclarator, _Parent:&mut dyn VisitMutWith) {
		if let Some(Ident { Sym, .. }) = Var.Name.AsIdent() {
			let Name = Sym.ToOwned();

			if let Some(Init) = &Var.Init {
				self.VarDefinitions.Insert(Name.Clone(), Init.Clone());

				self.VarUsage.Insert(Name, 0);
			}
		}
		Var.VisitMutChildrenWith(self);
	}

	fn VisitMutExpr(&mut self, Expr:&mut Expr, _Parent:&mut dyn VisitMutWith) {
		match Expr {
			Expr::Ident(Ident) => {
				let Name = Ident.Sym.ToOwned();

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
			},
			_ => {},
		}
		Expr.VisitMutChildrenWith(self);
	}

	fn VisitMutModuleItems(&mut self, Items:&mut Vec<ModuleItem>, _Parent:&mut dyn VisitMutWith) {
		Items.Retain(|Item| {
			if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(VarDecl))) = Item {
				for Decl in &VarDecl.Decls {
					if let Some(Ident { Sym, .. }) = Decl.Name.AsIdent() {
						let Name = Sym.ToOwned();

						if self.VarUsage.Get(&Name) == Some(&1) {
							self.Inlined = true;

							return false; // Remove this declaration
						}
					}
				}
			}
			true
		});
	}
}

use std::{
	collections::HashMap,
	fs,
	io::{self, Write},
	path::Path,
};

use rayon::prelude::*;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{Parser, StringInput, Syntax, lexer::Lexer};
use swc_ecma_visit::{VisitMut, VisitMutWith};
