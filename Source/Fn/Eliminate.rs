fn main() {
	let cm = SourceMap::default();

	let code = r#"
    let a = 5;
	
    let b = a + 1;
	
    let c = b * 2;
	
    console.log(c);
    "#;

	let fm = cm.new_source_file(FileName::Anon, code.into());

	let lexer = Lexer::new(
		Syntax::Typescript(Default::default()),
		Default::default(),
		StringInput::from(&*fm),
		None,
	);

	let mut parser = Parser::new_from(lexer);

	match parser.parse_module() {
		Ok(mut module) => {
			let mut inliner = Inliner::new(&cm);

			module.visit_mut_with(&mut inliner);

			let mut printer =
				swc_ecma_codegen::text_writer::JsWriter::new(cm.clone(), "\n", None, None);

			swc_ecma_codegen::node::module(&mut printer, &module).unwrap();

			println!("{}", printer.into_inner());
		},
		Err(err) => {
			eprintln!("Parsing error: {:?}", err);
		},
	}
}

struct Inliner<'a> {
	cm:&'a SourceMap,
	var_usage:HashMap<String, usize>,
	var_definitions:HashMap<String, Expr>,
}

impl<'a> Inliner<'a> {
	fn new(cm:&'a SourceMap) -> Self {
		Inliner { cm, var_usage:HashMap::new(), var_definitions:HashMap::new() }
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
							// Skip vars that are used only once
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

use std::collections::HashMap;

use swc_common::{SourceMap, Span};
use swc_ecma_ast::*;
use swc_ecma_parser::{Parser, StringInput, Syntax, lexer::Lexer};
use swc_ecma_visit::{VisitMut, VisitMutWith};
