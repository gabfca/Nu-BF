pub mod lexer
{
    use logos::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    pub enum TokenKind<'tk>
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

        /// Our additions.
        Identifier(&'tk str),

        #[regex(r"\$+[a-zA-Z]+", |lex| lex.slice())]
        Import(&'tk str), /* Import a routine  */

        #[regex(r"_+[a-zA-Z]+_", |lex| lex.slice())]
        Call(&'tk str), /* Call a routine & store returned value in current cell. */
    }

    pub struct LexedRoutine<'lr>
    {
        pub name: String,
        pub tokens: Vec<TokenKind<'lr>>,
    }

    impl<'lr> LexedRoutine<'lr>
    {
        pub fn new(name: &'lr str, contents: &'lr str) -> Self
        {
            let lex = TokenKind::lexer;
            let this_tokens: Vec<TokenKind> = lex(contents).collect();
            LexedRoutine { name: String::from(name.to_owned()), tokens: this_tokens.to_owned() }
        }
    }

    pub struct LexResult<'lre>
    {
        pub lexed_routines: Vec<LexedRoutine<'lre>>,
    }

    impl<'lre> LexResult<'lre>
    {
        pub fn new(routine_sources: &'lre [(&'lre str, &'lre str)]) -> Self
        {
            LexResult {
                lexed_routines: routine_sources
                    .iter()
                    .map(|(name, contents)| LexedRoutine::new(*name, *contents))
                    .collect(),
            }
        }
    }
}

pub mod parser
{

    use super::lexer::{LexResult, LexedRoutine, TokenKind};

    /// This is nubf's version of an "AST" "Node", which ends up being neither.
    /// The logical relationship between TokenKinds that this struct
    /// can represent is intentionally limited, in keeping with Classic Brainfuck's
    /// extremely simple, 'flat' grammar.
    #[derive(Clone)]
    pub struct TokenGrouping<'tkg>
    {
        pub parent: TokenKind<'tkg>,
        pub children: Option<Vec<TokenKind<'tkg>>>,
    }

    #[derive(Clone)]
    pub struct ParsedRoutine<'pr>
    {
        pub name: String,
        pub data: Vec<TokenGrouping<'pr>>,
    }

    impl<'pr> From<&'pr LexedRoutine<'pr>> for ParsedRoutine<'pr>
    {
        fn from(lexed: &'pr LexedRoutine) -> Self
        {
            let name = &lexed.name;
            let mut groupings: Vec<TokenGrouping> = Vec::new();
            let mut tokens_iter = lexed.tokens.iter();

            while let Some(token) = tokens_iter.next() {
                match token {
                    TokenKind::Import(ident_str) | TokenKind::Call(ident_str) => {
                        groupings.push(TokenGrouping {
                            parent: *token,
                            children: Some(Vec::from([TokenKind::Identifier(ident_str)])),
                        })
                    }

                    _ => groupings.push(TokenGrouping { parent: *token, children: None }),
                }
            }

            ParsedRoutine { name: name.to_string(), data: groupings.to_owned() }
        }
    }

    pub struct ParseResult<'pr>
    {
        pub parsed_routines: Vec<ParsedRoutine<'pr>>,
    }

    impl<'pr> ParseResult<'pr>
    {
        pub fn new(lexed_result: &'pr LexResult) -> Self
        {
            // All lexed routines.
            let routines = &lexed_result.lexed_routines;
            let routines_iter = routines.iter();

            // Parse all lexed routines.
            let parsed_routines =
                routines_iter.map(|routine| ParsedRoutine::from(routine)).collect::<Vec<_>>();
            Self { parsed_routines }
        }
    }
}
