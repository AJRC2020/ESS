use serde::Deserialize;

pub trait ServerConfig {
    fn name() -> &'static str;
    fn port(&self) -> u16;
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct GeneralConfig {
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthClientConfig {
    host: String,
    port: u16,
}

impl AuthClientConfig {
    pub fn authority(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// based on https://blog.logrocket.com/macros-in-rust-a-tutorial-with-examples/
#[macro_export]
macro_rules! server_config {
    (
     $server_name: literal,
     $(#[$meta:meta])*
     $vis:vis struct $struct_name:ident {
        $(
        $(#[$field_meta:meta])*
        $field_vis:vis $field_name:ident : $field_type:ty
        ),*$(,)+
    }
    ) => {
        $(#[$meta])*
        $vis struct $struct_name {
            general: ::server_common::GeneralConfig,
            $(
            $(#[$field_meta])*
            $field_vis $field_name : $field_type,
            )*
        }

        impl ::server_common::ServerConfig for $struct_name {
            fn name() -> &'static str {
                $server_name
            }

            fn port(&self) -> u16 {
                self.general.port
            }
        }
    }
}
