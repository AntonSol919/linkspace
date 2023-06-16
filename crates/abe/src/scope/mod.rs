use crate::eval::{EScope, EvalCtx};



pub mod base;
pub use base::BaseNScope;
pub mod logic;
pub use logic::LogicOps;
pub mod encode;
pub use encode::Encode;
pub mod help;
pub use help::Help;
pub mod argv;
pub use argv::ArgV;
pub mod comment;
pub use comment::Comment;
pub mod bytes;
pub use bytes::BytesFE;
pub mod uint;
pub use uint::UIntFE;

pub type EvalCore = (
    (EScope<BytesFE>, EScope<UIntFE>, EScope<BaseNScope>),
    ((EScope<Comment>,EScope<Help>), EScope<LogicOps>, EScope<Encode>),
);
pub type EvalCoreCtx = EvalCtx<EvalCore>;
pub const EVAL_SCOPE: EvalCore = core_scope();
pub const fn core_scope() -> EvalCore {
    (
        (EScope(BytesFE), EScope(UIntFE), EScope(BaseNScope)),
        ((EScope(Comment),EScope(Help)), EScope(LogicOps), EScope(Encode)),
    )
}
pub const fn core_ctx() -> EvalCoreCtx {
    EvalCtx {
        scope: core_scope()
    }
}
