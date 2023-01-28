// preopt.rs: General AST optimizations.

use crate::analysis::lexer::TokenKind;
use crate::analysis::parser::*;

#[derive(Debug)]
pub enum RepetitionOptimizable
{
    No,
    AsConsecutiveCount(u32),
    AsNonConsecutiveCount(u32)
}
use crate::RepetitionOptimizable::*;

#[derive(Debug)]
pub struct PreOptToken<'mt> 
{
    tok: Token<'mt>,
    
    is_repetition_optimizable: RepetitionOptimizable,
    is_extern_call: bool,
}

pub struct PreOptRoutine<'po>
{
    pub name: String,
    pub data: Vec<PreOptToken<'po>>
}

pub struct PreOptResult<'po>
{
    pub preopt_routines: Vec<PreOptRoutine<'po>>
}

impl<'pr> From<&'pr ParsedRoutine<'pr>> for PreOptRoutine<'pr>
{
    fn from(pr: &'pr ParsedRoutine) -> Self
    {       
        let mut preopt_toks: Vec<PreOptToken> = Vec::new();

        let mut tokens_iter = pr.data.iter();

        while let Some(token) = tokens_iter.next()
        {
            match token.kind 
            {
                TokenKind::Inc | TokenKind::Dec | TokenKind::Right | TokenKind::Left => 
                {   
                    let mut consecutive_uses = 1;
                    while let Some(ntok) = tokens_iter.next()
                    {
                        if ntok.kind == token.kind 
                        { consecutive_uses += 1; }
                    }

                    preopt_toks.push(PreOptToken { tok: token.to_owned(), is_repetition_optimizable: AsConsecutiveCount(consecutive_uses), is_extern_call: false})
                },

                TokenKind::BeginLoop | TokenKind::EndLoop => {
                    preopt_toks.push(PreOptToken { tok: token.to_owned(), is_repetition_optimizable: No, is_extern_call: false }) 
                },

                TokenKind::Call(some_fn_name) => 
                {
                    preopt_toks.push(
                        PreOptToken { 
                            tok: token.to_owned(), 
                            is_repetition_optimizable: No,
                            is_extern_call: false
                        }
                   ) 
                }, 
                _ => {}
            }

        }
        PreOptRoutine { name: pr.name.to_owned(), data: preopt_toks }
    }
}