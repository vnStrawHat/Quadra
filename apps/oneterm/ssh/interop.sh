#!/usr/bin/env bash
# Starts sshd on 127.0.0.1 with everything under .tmp/sshd-test/ and runs the live tests.
# Git Bash only; never connects anywhere else. Skips cleanly when sshd cannot start.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../../.."
root="$(pwd -W 2>/dev/null || pwd)"
dir=".tmp/sshd-test"
unix_dir="$PWD/$dir"
win_dir="$root/$dir"
port="${SSHD_TEST_PORT:-42923}"

if ! command -v /usr/bin/sshd >/dev/null || ! command -v /usr/bin/ssh-keygen >/dev/null; then
	echo "interop: no sshd in Git Bash, skipping"
	exit 0
fi

rm -rf "$dir"
mkdir -p "$dir/sftp"
gen() { /usr/bin/ssh-keygen -q "$@" -C quadra-test; }
gen -t ed25519 -N '' -f "$dir/hk_ed25519"
gen -t ecdsa -b 384 -N '' -f "$dir/hk_ecdsa"
gen -t rsa -b 3072 -N '' -f "$dir/hk_rsa"
gen -t ed25519 -N '' -f "$dir/id_ed25519"
gen -t ed25519 -N 'open sesame' -f "$dir/id_ed25519_pass"
gen -t ed25519 -N 'open sesame' -Z 'chacha20-poly1305@openssh.com' -f "$dir/id_ed25519_chacha"
gen -t ed25519 -N 'open sesame' -Z 'aes256-gcm@openssh.com' -f "$dir/id_ed25519_gcm"
gen -t rsa -b 2048 -N '' -f "$dir/id_rsa"
gen -t rsa -b 2048 -N 'open sesame' -f "$dir/id_rsa_pass"
gen -t rsa -b 2048 -m PEM -N '' -f "$dir/id_rsa_pem"
gen -t rsa -b 2048 -m PKCS8 -N '' -f "$dir/id_rsa_pkcs8"
gen -t ecdsa -b 256 -N '' -f "$dir/id_ecdsa256"
gen -t ecdsa -b 384 -N '' -f "$dir/id_ecdsa384"
gen -t ecdsa -b 521 -N '' -f "$dir/id_ecdsa521"
gen -t ecdsa -b 256 -m PEM -N '' -f "$dir/id_ecdsa_pem"
if command -v openssl >/dev/null; then
	openssl pkcs8 -topk8 -inform PEM -v2 aes-256-cbc -v2prf hmacWithSHA256 \
		-passout pass:'open sesame' -in "$dir/id_rsa_pem" -out "$dir/id_rsa_pkcs8_enc" 2>/dev/null
	cp "$dir/id_rsa_pem.pub" "$dir/id_rsa_pkcs8_enc.pub"
	openssl rsa -aes256 -passout pass:'open sesame' -in "$dir/id_rsa_pem" \
		-out "$dir/id_rsa_dek" 2>/dev/null
	cp "$dir/id_rsa_pem.pub" "$dir/id_rsa_dek.pub"
fi
cat "$dir"/id_*.pub >"$dir/authorized_keys"
for k in ed25519 ecdsa rsa; do
	/usr/bin/ssh-keygen -lf "$dir/hk_$k.pub" -E sha256 | cut -d' ' -f2 >"$dir/hk_$k.fp"
done

cat >"$dir/sshd_config" <<EOF
Port $port
ListenAddress 127.0.0.1
HostKey $unix_dir/hk_ed25519
HostKey $unix_dir/hk_ecdsa
HostKey $unix_dir/hk_rsa
PidFile $unix_dir/sshd.pid
AuthorizedKeysFile $unix_dir/authorized_keys
StrictModes no
UsePAM no
PasswordAuthentication yes
KbdInteractiveAuthentication yes
PubkeyAuthentication yes
AllowTcpForwarding yes
AllowAgentForwarding yes
GatewayPorts no
RekeyLimit 8M
LogLevel DEBUG1
Subsystem sftp /usr/lib/ssh/sftp-server
EOF

/usr/bin/sshd -D -e -f "$unix_dir/sshd_config" >"$dir/sshd.log" 2>&1 &
pid=$!
trap 'kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null' EXIT

# wait until the port answers (bash /dev/tcp), at most about 5 s
for _ in $(seq 50); do
	if (exec 3<>/dev/tcp/127.0.0.1/"$port") 2>/dev/null; then
		break
	fi
	sleep 0.1
done
if ! kill -0 "$pid" 2>/dev/null; then
	echo "interop: sshd did not start, skipping; see $dir/sshd.log"
	exit 0
fi

# a sanity check of the sshd config with the stock client, independent of our code
/usr/bin/sftp -q -F /dev/null -P "$port" -i "$dir/id_ed25519" -o StrictHostKeyChecking=no \
	-o UserKnownHostsFile=/dev/null -o BatchMode=yes -b /dev/null 127.0.0.1 ||
	echo "interop: warning: the stock sftp client could not connect"

export QUADRA_SSHD_TESTS=1 SSHD_PORT="$port" SSHD_DIR="$win_dir"
exec_test=(tools/v.sh test "$@")
if [ $# -eq 0 ]; then
	exec_test=(tools/v.sh test apps/oneterm/ssh)
fi
if [ -n "${SSHD_RUN:-}" ]; then # a debug build of a test: SSHD_RUN=.tmp/x.exe interop.sh
	$SSHD_RUN
	exit $?
fi
"${exec_test[@]}"
