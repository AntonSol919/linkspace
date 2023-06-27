


#[macro_export]
macro_rules! build_info {
    () => {
        pub static BUILD_INFO : &'static str = concat!(
            env!("CARGO_PKG_NAME")," - ",
            env!("CARGO_PKG_VERSION")," - ",
            env!("VERGEN_GIT_BRANCH")," - ",
            env!("VERGEN_GIT_DESCRIBE"), " - ", 
            env!("VERGEN_RUSTC_SEMVER")
        );
    };
}

build_info!();
