fn main() -> Result<(), Box<dyn std::error::Error>> {
	let Paths = vec![
		Path::New("file1.ts"),
		Path::New("file2.ts"),
		// Add more paths here
	];

	Paths.ParIter().ForEach(|Path| {
		match ProcessFileRecursive(Path) {
			Ok(Content) => {
				fs::Write(Path, Content).Expect("Unable to write file");

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

	let Code = fs::ReadToString(Path)?;

	let Fm = Cm.NewSourceFile(FileName::Real(Path.ToPathBuf()), Code.Into());

	let Lexer = Lexer::New(
		Syntax::Typescript(Default::default()),
		Default::default(),
		StringInput::From(&*Fm),
		None,
	);

	let mut Parser = Parser::NewFrom(Lexer);

	let mut Module = Parser.ParseModule()?;

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

		swc_ecma_codegen::node::module(&mut Printer, &Module)?;

		Buf = Printer.IntoInner();
	}
	Ok(String::FromUtf8(Buf)?)
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
		self.Inlined = false; // Reset Inlined flag
		Module.VisitMutWith(self);

		Module
	}
}

impl<'a> VisitMut for Inliner<'a> {
	fn VisitMutVarDeclarator(&mut self, Var:&mut VarDeclarator, Parent:&mut dyn VisitMutWith) {
		if let Some(Ident { Sym, .. }) = Var.Name.AsIdent() {
			let Name = Sym.ToString();

			if let Some(Init) = &Var.Init {
				self.VarDefinitions.Insert(Name.Clone(), Init.Clone());

				self.VarUsage.Insert(Name, 0);
			}
		}
		Var.VisitMutChildrenWith(self);
	}

	fn VisitMutExpr(&mut self, Expr:&mut Expr, Parent:&mut dyn VisitMutWith) {
		match Expr {
			Expr::Ident(Ident) => {
				let Name = Ident.Sym.ToString();

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

	fn VisitMutModuleItems(&mut self, N:&mut Vec<ModuleItem>, Parent:&mut dyn VisitMutWith) {
		let mut Items = Vec::new();

		for Item in N.Drain(..) {
			if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(VarDecl))) = &Item {
				for Decl in VarDecl.Decls.Iter() {
					if let Some(Ident { Sym, .. }) = Decl.Name.AsIdent() {
						let Name = Sym.ToString();

						if let Some(&1) = self.VarUsage.Get(&Name) {
							self.Inlined = true;

							continue;
						}
					}
				}
			}
			Items.Push(Item);
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
