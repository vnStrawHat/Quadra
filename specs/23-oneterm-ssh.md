# 23: the OneTerm SSH and SFTP engine

Status: implemented in `apps/oneterm/ssh/` (V `module ssh`, imported from the app as `import ssh`).
A 1:1 feature port of the Rust OneTerm 0.6.11 `crates/ssh` engine, without its UI.

## Decisions

| # | Decision | Why |
|---|---|---|
| D1 | Pure V SSH2 over vlib primitives. No third-party C, no `#flag -l`. | `libssh2` over Windows CNG has no ed25519. |
| D2 | Windows CNG (`bcrypt.dll`, loaded at run time) does RSA, ECDSA, ECDH and AES (ECB for CTR mode, GCM). chacha20, poly1305, curve25519, ed25519, SHA-2, HMAC and blowfish come from vlib. | vlib has no RSA or P-curves, and its AES is table based (not constant time) with a bit-by-bit GHASH (slow). CNG is constant time and AES-NI backed. |
| D3 | One reader thread per connection owns decryption and dispatch. Writes take one mutex that guards the socket and the send cipher. Everything is blocking on worker threads. | Replaces the Rust async runtime with the smallest correct model. |
| D4 | A `Stream` interface (TCP, or a direct-tcpip channel of another connection) carries the transport, so jump hosts need no special path. | One code path for the first hop and every later one. |
| D5 | The reader thread never waits for a consumer. Channel data is buffered up to the channel window; each consumer (shell pump, SFTP demux, forward relay) has its own thread. | A callback that writes back would otherwise wait for a window update only the reader can deliver. |
| D6 | `connect` runs the whole route, opens the shell, SFTP and forwards on the caller's worker thread. `Session.attach(opts)` only starts delivering, so it can be `term.start`'s spawn function on the UI thread. | Same order as the Rust `connect`; the network never runs on the UI thread. |
| D7 | `Pty.write` never blocks: it enqueues under a 4 MiB byte budget and fails when full. A writer thread drains the queue; resizes coalesce; close is honoured out of band. | Same contract as the Rust `SshTransport`. |
| D8 | Secrets live in `Secret` (zeroed on `wipe`, printed as `***`). Session keys are wiped on rekey and on close. MAC comparison is constant time. Nothing logs a secret, a key or a payload. | |
| D9 | A closed shell closes the whole session (SFTP, forwards, relays, jump hops, target first and hops innermost-out). | Same as the Rust teardown block. |

## Parity notes (where the port differs, on purpose)

- **`mlkem768x25519-sha256` is not offered.** vlib has no ML-KEM. `curve25519-sha256` is first.
- **Compression is `none` only.** The Rust engine offers `none` first, so a server only picks `zlib` when it has no `none`; no OpenSSH configuration does that.
- **Resume does not exist in the Rust engine** (`.part` sibling, replaced only when complete; a retry starts at byte 0), so none here.
- **Unix `$SSH_AUTH_SOCK` is not supported yet**: this build is Win32 only, like the rest of Quadra. The pipe name is a `Config` field so tests can use their own.
- **`known_hosts_path` is a required `Config` field**: the engine never looks at the home directory on its own; the app passes `~/.ssh/known_hosts`.
- Bandwidth counters count shell and SFTP payload bytes, like the Rust `SharedState` counters.
- **The bash `PS0` mark ends with BEL, not `ESC \`** (the bootstrap of the original uses `__oneterm_mark C`). A `PS0` value ending in a lone backslash makes bash 4.4 (Git Bash) print garbage bytes from past the end of the prompt string before every command's output.

## Protocol surface

Client version string: `SSH-2.0-OneTerm_V_0.6.11`.

| Area | Offered, in order |
|---|---|
| Key exchange | `curve25519-sha256`, `curve25519-sha256@libssh.org`, `ecdh-sha2-nistp256`, `ecdh-sha2-nistp384`, `ecdh-sha2-nistp521`, `diffie-hellman-group-exchange-sha256`, `diffie-hellman-group16-sha512`, `diffie-hellman-group18-sha512`, `diffie-hellman-group14-sha256`, plus `ext-info-c` and `kex-strict-c-v00@openssh.com` |
| Host key | `ssh-ed25519`, `ecdsa-sha2-nistp256/384/521`, `rsa-sha2-512`, `rsa-sha2-256`, `ssh-rsa`; algorithms already recorded in known_hosts are moved to the front |
| Cipher | `chacha20-poly1305@openssh.com`, `aes256-gcm@openssh.com`, `aes128-gcm@openssh.com`, `aes256-ctr`, `aes192-ctr`, `aes128-ctr` |
| MAC (for CTR) | `hmac-sha2-512-etm@openssh.com`, `hmac-sha2-256-etm@openssh.com`, `hmac-sha2-512`, `hmac-sha2-256`, `hmac-sha1-etm@openssh.com`, `hmac-sha1` |
| Compression | `none` |

Rekey: on a server KEXINIT at any time, and by the client after 1 GiB sent or received or one hour. Strict key exchange is honoured (sequence numbers reset at NEWKEYS, unexpected packets are fatal during the first exchange).

## Authentication

`none`; `password` with the keyboard-interactive fallback (one round, every prompt answered with the password, an echoing prompt or a second round is a clear error); public key from a file; the SSH agent. Failure messages name the remaining methods exactly as the Rust engine does.

Key files (the same set the Rust loader reads): OpenSSH (`-----BEGIN OPENSSH PRIVATE KEY-----`, cipher `none`, `aes128/192/256-ctr`, `aes128/256-cbc`, `aes128/256-gcm@openssh.com`, `chacha20-poly1305@openssh.com` with `bcrypt` KDF), PKCS#1 (`BEGIN RSA PRIVATE KEY`, legacy `DEK-Info` AES-128/256-CBC), SEC1 (`BEGIN EC PRIVATE KEY`), PKCS#8 (`BEGIN PRIVATE KEY`, and `BEGIN ENCRYPTED PRIVATE KEY` as PBES2 + PBKDF2-HMAC-SHA1/SHA256 + AES-CBC). Key types: ed25519, RSA, ECDSA P-256/384/521. RSA never signs with SHA-1: the hash comes from `server-sig-algs` (`rsa-sha2-512` if listed, else `rsa-sha2-256`, else SHA-512).

Agent: the pipe `\\.\pipe\openssh-ssh-agent`, then Pageant (`WM_COPYDATA`). At most 6 identities, stops when the server drops `publickey`. Agent forwarding is per session: requested on the session channel before the PTY, a refusal is one warning; server-opened agent channels are refused unless it is on, and bridged per message when it is.

## Host keys

`known_hosts` in OpenSSH format (plain, hashed `|1|`, comma lists, `[host]:port` when the port is not 22, wildcards and `!` negation, `@revoked`). The verdicts are the Rust table: exact match proceeds; no entry is `HostKeyUnknown` (fails closed); same algorithm with another key, or only other algorithms recorded, is `HostKeyChanged`; a `@revoked` match is `HostKeyChanged`; an unreadable file or malformed matching entry is a connect error; a certificate is a connect error. `HostKeyPolicy.accept_new(fingerprint)` trusts and appends exactly that SHA-256 fingerprint once.

## Connections

Phases with the Rust deadlines: 20 s per phase, 60 s overall, cancellable from any thread (`Cancel`). `ConnectError{phase, message}`, `HostKeyUnknown`, `HostKeyChanged`, `Cancelled` are `IError` types the UI can match. Route: at most 4 jump hops, each with its own credential and host-key policy, errors prefixed `jump host <label>: ...` except host-key errors and cancellation. Keepalive: `keepalive@openssh.com` every 30 s (5..3600), disconnect after 3 (1..20) unanswered; any traffic counts as an answer.

Shell: `xterm-256color`, optional `ECHO 0` mode for shell integration, env requests `COLORTERM`, `TERM_PROGRAM`, `TERM_PROGRAM_VERSION`, the bootstrap line from the Rust engine typed after the shell opens.

Forwards (`Local`, `Remote` with server-chosen ports, `Dynamic` SOCKS5 no-auth CONNECT with IPv4, IPv6 and domain): a forward that cannot start is one warning, the shell stays up; a `forwarded-tcpip` for an address not requested is refused.

## SFTP

Version 3 over a `sftp` subsystem channel with an 8 MiB window. One demux thread matches replies to requests, so any number of worker threads can call it.

Operations: `read_dir` (folders first, then case-insensitive names, no `.`/`..`), `stat`, `realpath`, `rename`, `remove`, `remove_dir_all`, `mkdir`, `upload`, `download`, `cancel_transfer`, `close`, `alive`; owner and group names from `/etc/passwd` and `/etc/group` (4 MiB cap, best effort).

Transfers: 255 KiB chunks clamped to `limits@openssh.com`, 16 reads or 8 writes in flight, one pass per file, `.part` sibling then backup-and-rename, permissions and times copied best effort, depth 64 and 100 000 entries caps, folder trees listed first for a true progress denominator, progress at most once per chunk, duplicate transfer ids refused. Remote names are validated before they become local paths; remote and local symlinks are refused on transfers; `remove_dir_all` unlinks symlinks and never follows them.

## Files

| File | Holds |
|---|---|
| `config.v` | `Config`, `Hop`, auth, host-key policy, keepalive, forwards, `Secret` |
| `errors.v` | error types, `Cancel` |
| `wire.v` | SSH wire reader and writer |
| `stream.v` | `Stream`, the TCP stream |
| `quadra/native/cng` (`cng_windows.c.v`, `cng_keys_windows.c.v`, `keys.v`) | the bcrypt.dll wrappers, outside this module so the app stays free of OS code |
| `seal.v`, `seal_aes.v` | packet protection per cipher |
| `transport.v`, `kex.v`, `kex_run.v`, `kex_groups.v` | packets, negotiation, exchange, rekey |
| `hostkey.v`, `known_hosts.v` | key blobs, signatures, verdicts |
| `keyfile.v`, `keyfile_pem.v`, `der.v`, `bcrypt_pbkdf.v`, `signer.v` | private keys |
| `auth.v`, `agent.v`, `agent_link.v` | authentication, agent; the pipe and Pageant transports are in `quadra/native/sys/agent_windows.c.v` |
| `conn.v`, `conn_dispatch.v`, `channel.v`, `chan_stream.v` | connection protocol |
| `shell.v`, `bootstrap.v` | `platform.Pty` implementation |
| `session.v`, `route.v` | `connect`, phases, route, teardown |
| `forward.v`, `socks5.v` | port forwards |
| `sftp.v`, `sftp_wire.v`, `sftp_ops.v`, `sftp_meta.v`, `sftp_paths.v`, `sftp_copy.v`, `sftp_upload.v`, `sftp_download.v`, `sftp_stage.v` | SFTP |
| `*_test.v`, `test_support.v`, `interop.sh` | tests (`test_support.v` holds helpers shared by the test files; nothing else calls them) |

## Tests

`tools/v.sh test apps/oneterm/ssh` runs the table tests. `apps/oneterm/ssh/interop.sh` starts sshd on 127.0.0.1 with everything under `.tmp/sshd-test/` and runs the same tests with `QUADRA_SSHD_TESTS=1`, which turns on the live tests. Never against any other host.
