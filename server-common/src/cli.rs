use std::path::Path;

pub trait ServerArgs {
    fn port(&self) -> Option<u16>;
    fn config_path(&self) -> &Path;
}

#[macro_export]
macro_rules! server_args {
    ($default_cfg_path: literal) => {
        #[derive(Debug, ::server_common::prelude::clap::Parser)]
        struct Args {
            /// Port to listen on
            #[arg(short, long)]
            port: Option<u16>,

            /// Path to config file
            #[arg(short, long, default_value = $default_cfg_path)]
            config: std::path::PathBuf,
        }

        impl ::server_common::ServerArgs for Args {
            fn port(&self) -> Option<u16> {
                self.port
            }

            fn config_path(&self) -> &std::path::Path {
                &self.config
            }
        }
    };
}
