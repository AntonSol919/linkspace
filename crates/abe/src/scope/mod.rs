use crate::eval::EScope;

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

pub type BasicScope = (
    (EScope<BytesFE>, EScope<UIntFE>, EScope<BaseNScope>),
    (
        (EScope<Comment>, EScope<Help>),
        EScope<LogicOps>,
        EScope<Encode>,
    ),
);
pub const fn basic_scope() -> BasicScope {
    (
        (EScope(BytesFE), EScope(UIntFE), EScope(BaseNScope)),
        (
            (EScope(Comment), EScope(Help)),
            EScope(LogicOps),
            EScope(Encode),
        ),
    )
}
pub static BASIC_SCOPE: BasicScope = basic_scope();
