use abe::eval::{EvalScopeImpl, ScopeFunc, ScopeFuncInfo, none};

use super::LKS;

#[derive(Copy, Clone)]
pub struct FileEnv<R>(pub (crate) R);

impl<R: LKS> EvalScopeImpl for FileEnv<R> {
    fn about(&self) -> (String, String) {
        (
            "filesystem env".into(),
            format!(
                "read files from {:?}",
                self.0.lk().map(|v| v.env().files_path())
        )
    )
}
fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
    &[ScopeFunc {
        apply: |this: &Self, inp: &[&[u8]], _, _scope| {
            let p = std::str::from_utf8(inp[0])?;
            Ok(this.0.lk()?.env().files_data(p,true)?.unwrap()).into()
        },
        info: ScopeFuncInfo {
            id: "files",
            init_eq: None,
            argc: 1..=1,
            to_abe: false,
            help: "read a file from the LK_DIR/files directory",
        },
        to_abe: none,
    }]
}
}
