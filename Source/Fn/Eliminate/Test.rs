
// Helper function to generate output from AST
fn generate_output(Cm: &SourceMap, Module: &Module) -> String {
    let mut Writer = Vec::new();
    {
        let mut Emitter = Emitter {
            Cfg: Config { minify: false },
            Cm: Cm.Clone(),
            Comments: None,
            Wr: Box::new(JsWriter::new(Cm.Clone(), "\n", None, None)),
        };
        Emitter.EmitModule(Module).expect("Failed to emit module");
        Writer = Emitter.Wr.into_inner();
    }
    String::FromUtf8(Writer).expect("Failed to convert output to UTF-8")
}

#[test]
fn test_inline_single_use_variable() {
    let Cm = SourceMap::new(FilePathMapping::empty());
    let Code = "let a = 5; let b = a + 1;".to_string();
    let Fm = Cm.NewSourceFile(FileName::Anon, Code);

    let Lexer = Lexer::new(
        Syntax::Typescript(Default::default()),
        Default::default(),
        StringInput::from(&*Fm),
        None,
    );
    let mut Parser = Parser::new_from(Lexer);

    let Module = Parser.ParseModule().unwrap();
    let mut Inliner = Inliner::New(&Cm);
    let InlinedModule = Inliner.Inline(Module);

    let mut Writer = Vec::new();
    {
        let mut Emitter = Emitter {
            Cfg: Config { minify: false },
            Cm: Cm.Clone(),
            Comments: None,
            Wr: Box::new(JsWriter::new(Cm.Clone(), "\n", None, None)),
        };
        Emitter.EmitModule(&InlinedModule).unwrap();
        Writer = Emitter.Wr.into_inner();
    }

    let Result = String::FromUtf8(Writer).unwrap();
    assert_eq!(Result, "let b = 5 + 1;");
}

#[test]
fn test_do_not_inline_exported_variable() {
    let Cm = SourceMap::new(FilePathMapping::empty());
	
    let Code = "export let a = 5; let b = a + 1;".to_string();
	
    let Fm = Cm.NewSourceFile(FileName::Anon, Code);
	

    let Lexer = Lexer::new(
        Syntax::Typescript(Default::default()),
        Default::default(),
        StringInput::from(&*Fm),
        None,
    );
	
    let mut Parser = Parser::new_from(Lexer);
	

    let Module = Parser.ParseModule().unwrap();
	
    let mut Inliner = Inliner::New(&Cm);
	
    let InlinedModule = Inliner.Inline(Module);
	

    let mut Writer = Vec::new();
	
    {
        let mut Emitter = Emitter {
            Cfg: Config { minify: false },
            Cm: Cm.Clone(),
            Comments: None,
            Wr: Box::new(JsWriter::new(Cm.Clone(), "\n", None, None)),
        };
		
        Emitter.EmitModule(&InlinedModule).unwrap();
		
        Writer = Emitter.Wr.into_inner();
		
    }

    let Result = String::FromUtf8(Writer).unwrap();
	
    assert_eq!(Result, "export let a = 5;\nlet b = a + 1;");
	
}

#[test]
fn test_inline_recursive() -> io::Result<()> {
    let TempFile = NamedTempFile::new()?;
	
    let Path = TempFile.Path();
	
    fs::Write(Path, "let a = 5; let b = a + 1; let c = b * 2;")?;
	

    ProcessFileRecursive(Path)?;
	
    let Result = fs::ReadToString(Path)?;
	
    assert_eq!(Result, "let c = 5 + 1 * 2;");
	

    Ok(())
}
