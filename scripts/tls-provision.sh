#!/usr/bin/env bash
set -euo pipefail

readonly DEFAULT_VALIDITY_DAYS=365

usage() {
    cat <<'EOF'
Usage: tls-provision.sh --output DIRECTORY --host IP_OR_DNS [--host IP_OR_DNS ...]

Create a new self-signed TLS leaf certificate for the AnalogConnect daemon.
The output directory must not exist and must be outside the project repository.
Enter the printed SHA-256 pin in the Android enrollment screen.
EOF
}

fail() {
    printf 'TLS_PROVISION=FAILED reason=%s\n' "$1" >&2
    exit 1
}

is_ipv4() {
    local host=$1 part
    [[ "$host" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]] || return 1
    IFS=. read -r -a parts <<<"$host"
    for part in "${parts[@]}"; do
        [[ "$part" =~ ^[0-9]{1,3}$ ]] || return 1
        ((10#$part <= 255)) || return 1
    done
}

is_dns_name() {
    local host=$1 label
    [[ ${#host} -le 253 && "$host" == *.* ]] || return 1
    IFS=. read -r -a labels <<<"$host"
    for label in "${labels[@]}"; do
        [[ ${#label} -ge 1 && ${#label} -le 63 ]] || return 1
        [[ "$label" =~ ^[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?$ ]] || return 1
    done
}

output_dir=
declare -a hosts=()
while (($#)); do
    case "$1" in
        --output)
            (($# >= 2)) || { usage >&2; exit 64; }
            output_dir=$2
            shift 2
            ;;
        --host)
            (($# >= 2)) || { usage >&2; exit 64; }
            hosts+=("$2")
            shift 2
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            usage >&2
            exit 64
            ;;
    esac
done

[[ -n "$output_dir" && ${#hosts[@]} -gt 0 ]] || { usage >&2; exit 64; }
for command in openssl realpath sed; do
    command -v "$command" >/dev/null 2>&1 || fail "required_helper_unavailable"
done

project_root=$(realpath "$(dirname -- "${BASH_SOURCE[0]}")/..")
resolved_output=$(realpath -m -- "$output_dir")
case "$resolved_output" in
    "$project_root"|"$project_root"/*) fail "output_inside_repository" ;;
esac
[[ ! -e "$resolved_output" ]] || fail "output_already_exists"

declare -a sans=()
for host in "${hosts[@]}"; do
    if is_ipv4 "$host"; then
        sans+=("IP:$host")
    elif [[ "$host" =~ ^[0-9.]+$ ]]; then
        fail "invalid_host"
    elif is_dns_name "$host"; then
        sans+=("DNS:$host")
    else
        fail "invalid_host"
    fi
done
san_list=$(IFS=,; printf '%s' "${sans[*]}")

umask 077
mkdir -p -- "$(dirname -- "$resolved_output")"
mkdir -- "$resolved_output"
key_path="$resolved_output/daemon-key.pem"
cert_path="$resolved_output/daemon-cert.pem"
openssl req -x509 -newkey rsa:3072 -sha256 -nodes \
    -days "$DEFAULT_VALIDITY_DAYS" \
    -keyout "$key_path" -out "$cert_path" \
    -subj "/CN=${hosts[0]}" -addext "subjectAltName=$san_list" \
    >/dev/null 2>&1 || fail "certificate_generation_failed"
chmod 0600 "$key_path"
chmod 0644 "$cert_path"

pin=$(openssl x509 -in "$cert_path" -outform DER 2>/dev/null |
    openssl dgst -sha256 -hex 2>/dev/null |
    sed -n 's/^.*= //p')
[[ "$pin" =~ ^[0-9a-f]{64}$ ]] || fail "pin_generation_failed"

printf 'TLS_PROVISION=PASS\n'
printf 'CERTIFICATE_PATH=%s\n' "$cert_path"
printf 'PRIVATE_KEY_PATH=%s\n' "$key_path"
printf 'CERTIFICATE_SHA256=%s\n' "$pin"
