#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
mkdir -p "${SCRIPT_DIR}/cfg/tls"
cd "${SCRIPT_DIR}/cfg/tls"

openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:4096 -out root_ca.key
openssl x509 -new -key root_ca.key -days 7 -subj "/C=PT/ST=Porto/L=Porto/O=FEUP/OU=ESS 2023/CN=Root CA/emailAddress=." -extfile <(echo "basicConstraints=CA:TRUE") -out root_ca.cert

generate_certificate() {
    openssl req -new -sha256 -newkey rsa:4096 -nodes -subj "/C=PT/ST=Porto/L=Porto/O=FEUP/OU=ESS 2023/CN=$1/emailAddress=." -keyout "$1.key" -out "$1.req"
    openssl x509 -req -in "$1.req" -days 7 -CA root_ca.cert -CAkey root_ca.key -CAcreateserial -extfile <(echo -e "authorityKeyIdentifier=keyid,issuer\nbasicConstraints=CA:FALSE\nsubjectAltName=DNS:localhost") -out "$1.cert"
    rm "$1.req"
}

generate_certificate "app-server"
generate_certificate "auth-server"
generate_certificate "service-fileshare"
generate_certificate "service-filestore"
