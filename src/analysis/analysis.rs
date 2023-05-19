pub(crate) mod lexer
{

    use logos::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    pub(crate) enum TokenKind<'tk>
    {
        // Standard Brainfuck
        #[token("+")]
        Inc,
        #[token("-")]
        Dec,
        #[token(">")]
        Right,
        #[token("<")]
        Left,
        #[token(",")]
        Input,
        #[token(".")]
        Output,
        #[token("[")]
        BeginLoop,
        #[token("]")]
        EndLoop,
        #[error]
        Error,

        #[token(" ")]
        Space,
        #[token("\n")]
        Newline,

        // Our additions.
        Identifier(&'tk str),

        #[regex(r"\$+[a-zA-Z]+", |lex| lex.slice())]
        Import(&'tk str), // Import a routine.

        #[regex(r"_+[a-zA-Z]+_", |lex| lex.slice())]
        Call(&'tk str), // Call a routine & store returned value in current cell.
    }

    pub(crate) struct LexedRoutine<'lr>
    {
        pub(crate) name: String,
        pub(crate) tokens: Vec<TokenKind<'lr>>,
    }

    impl<'lr> LexedRoutine<'lr>
    {
        pub(crate) fn new(name: String, contents: &'lr str) -> Self
        {
            let lex = TokenKind::lexer;
            let this_tokens: Vec<TokenKind> = lex(contents).collect();
            LexedRoutine { name, tokens: this_tokens.to_owned() }
        }
    }

    pub(crate) struct LexedProgram<'lre>
    {
        pub(crate) routines: Vec<LexedRoutine<'lre>>,
    }

    impl<'lp> LexedProgram<'lp>
    {
        pub(crate) fn new(routine_sources: &'lp [(String, String)]) -> Self
        {
            LexedProgram {
                routines: routine_sources
                    .iter()
                    .map(|(name, contents)| LexedRoutine::new(name.clone(), contents))
                    .collect(),
            }
        }
    }
}

pub(crate) mod parser
{
    use super::lexer::*;

    /// This is nubf's version of an "AST" "Node", which ends up being neither.
    /// The logical relationship between TokenKinds that this struct
    /// can represent is intentionally limited, in keeping with Classic Brainfuck's
    /// extremely simple, 'flat' grammar.
    #[derive(Clone, Debug)]
    pub(crate) struct Token<'tkg>
    {
        pub(crate) kind: TokenKind<'tkg>,
        pub(crate) children: Option<Vec<TokenKind<'tkg>>>,
    }

    #[derive(Clone)]
    pub(crate) struct ParsedRoutine<'pr>
    {
        pub(crate) name: String,
        pub(crate) data: Vec<Token<'pr>>,
    }

    impl<'pr> From<&'pr LexedRoutine<'pr>> for ParsedRoutine<'pr>
    {
        fn from(lexed: &'pr LexedRoutine) -> Self
        {
            let name = &lexed.name;
            let mut groupings: Vec<Token> = Vec::new();
            let tokens_iter = lexed.tokens.iter();

            for token in tokens_iter {
                match token {
                    TokenKind::Import(ident_str) | TokenKind::Call(ident_str) => {
                        groupings.push(Token {
                            kind: *token,
                            children: Some(Vec::from([TokenKind::Identifier(ident_str)])),
                        })
                    }

                    _ => groupings.push(Token { kind: *token, children: None }),
                }
            }

            ParsedRoutine { name: name.to_string(), data: groupings.to_owned() }
        }
    }

    pub(crate) struct ParsedProgram<'pp>
    {
        pub(crate) routines: Vec<ParsedRoutine<'pp>>,
    }

    impl<'pr> From<&'pr LexedProgram<'pr>> for ParsedProgram<'pr>
    {
        fn from(lr: &'pr LexedProgram<'pr>) -> Self
        {
            // All lexed routines.
            let routines = &lr.routines;
            let routines_iter = routines.iter();

            // Parse all lexed routines.
            let parsed_routines = routines_iter.map(ParsedRoutine::from).collect::<Vec<_>>();
            Self { routines: parsed_routines }
        }
    }
}
