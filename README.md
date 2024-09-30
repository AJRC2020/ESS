# CipherShare

Final Project

## Running

Run `./generate-certificates.sh` to generate the required TLS certificates. The `cfg/tls/root_ca.cert` certificate should be added to the browser as a certificate authority to avoid HTTPS and CORS problems.

Make sure you have a recent [Rust toolchain](https://www.rust-lang.org/tools/install) installed, then run

* `cargo run -p app-server`
* `cargo run -p auth-server`
* `cargo run -p service-fileshare`
* `cargo run -p service-filestore`

to run the various executables.

---

ESS 2023 - Group 2:

* Alberto Cunha (201906325)
* Fernando Rocha (202200589)
* João Silva (201906478)
* Joaquim Monteiro (201905257)

Presentation Slides available [here](./docs/ESS-2023-Apresentaçao.pdf).
