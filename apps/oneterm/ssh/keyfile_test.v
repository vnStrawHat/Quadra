// Private key parsing against real ssh-keygen output (throwaway keys, used nowhere else).
module ssh

import encoding.base64

const ed25519_plain = '-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACDyXPDhf/OuApldCOUaTBr4UTetxzZZmoYxabRAgodVuAAAAJDcGrMq3Bqz
KgAAAAtzc2gtZWQyNTUxOQAAACDyXPDhf/OuApldCOUaTBr4UTetxzZZmoYxabRAgodVuA
AAAEASbTO1FIE4hB8j38IzaEj32vv1DpQWcm0J2ia2xtaArPJc8OF/864CmV0I5RpMGvhR
N63HNlmahjFptECCh1W4AAAAB2ZpeHR1cmUBAgMEBQY=
-----END OPENSSH PRIVATE KEY-----
'

const ed25519_encrypted = '-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAACmFlczI1Ni1jdHIAAAAGYmNyeXB0AAAAGAAAABA6gp1xgE
SYlEDVBOpDVQR+AAAAEAAAAAEAAAAzAAAAC3NzaC1lZDI1NTE5AAAAIKoVi2rlYwaW7mP7
yYgA+LgDGNJk1IuUqGwdxn1leIyNAAAAkI/dXBjauc/Cw7pXq7bSLCcMiyDwMDMPr+n02n
Bw3EFGHEkqjmrAM7TQh2le75e2ZaTk03YeJs2jSNXWO0kn2436PtS9rz0798r3Gajxg7F6
ZBaZI6ZBZokwi7jM1ldKBORk3Ek+NI4lhthPJNHpFXU93H2siSafu3pLDOsvK7PgjCv163
OvouE3hTUJBpig0Q==
-----END OPENSSH PRIVATE KEY-----
'

const ecdsa_plain = '-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAaAAAABNlY2RzYS
1zaGEyLW5pc3RwMjU2AAAACG5pc3RwMjU2AAAAQQRTXC1460rg9o49kjAH3HfbA04BXDap
jnHcyxGfEksuZK/pSeZJudWmeJx4cpPy1Rn/h0SXwI5fyfIWQCgbV5QGAAAAoGPKLK9jyi
yvAAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBFNcLXjrSuD2jj2S
MAfcd9sDTgFcNqmOcdzLEZ8SSy5kr+lJ5km51aZ4nHhyk/LVGf+HRJfAjl/J8hZAKBtXlA
YAAAAhAOQdWdJaJIYEBCpSIAXZ8kuZJQJrtrvJzCi8MyiNNdTiAAAAB2ZpeHR1cmU=
-----END OPENSSH PRIVATE KEY-----
'

const ed25519_plain_pub = 'AAAAC3NzaC1lZDI1NTE5AAAAIPJc8OF/864CmV0I5RpMGvhRN63HNlmahjFptECCh1W4'
const ed25519_encrypted_pub = 'AAAAC3NzaC1lZDI1NTE5AAAAIKoVi2rlYwaW7mP7yYgA+LgDGNJk1IuUqGwdxn1leIyN'
const ecdsa_pub = 'AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBFNcLXjrSuD2jj2SMAfcd9sDTgFcNqmOcdzLEZ8SSy5kr+lJ5km51aZ4nHhyk/LVGf+HRJfAjl/J8hZAKBtXlAY='

fn test_ed25519_plain_key() {
	key := decode_private_key(ed25519_plain, none) or { panic(err) }
	assert key.kind == 'ssh-ed25519'
	assert base64.encode(key.public_blob) == ed25519_plain_pub
	assert fingerprint(key.public_blob) == 'SHA256:QEWei2+FbNmeR5eIBM/l2A2sId9lhve/3133QScroYo'
}

fn test_encrypted_key_uses_bcrypt_pbkdf() {
	key := decode_private_key(ed25519_encrypted, secret('test-pass')) or { panic(err) }
	assert base64.encode(key.public_blob) == ed25519_encrypted_pub
	if _ := decode_private_key(ed25519_encrypted, secret('nope')) {
		assert false, 'a wrong passphrase must fail'
	}
	if _ := decode_private_key(ed25519_encrypted, none) {
		assert false, 'a missing passphrase must fail'
	}
}

fn test_ecdsa_key_signs_and_verifies() {
	key := decode_private_key(ecdsa_plain, none) or { panic(err) }
	assert key.kind == 'ecdsa-sha2-nistp256'
	assert base64.encode(key.public_blob) == ecdsa_pub
	data := 'sign me'.bytes()
	sig := key.sign(key.kind, data) or { panic(err) }
	host := parse_host_key(key.public_blob) or { panic(err) }
	host.verify_sig(key.kind, data, sig) or { panic(err) }
	if _ := host.verify_sig(key.kind, 'other'.bytes(), sig) {
		assert false, 'verification must fail on other data'
	}
}

fn test_ed25519_key_signs_and_verifies() {
	key := decode_private_key(ed25519_plain, none) or { panic(err) }
	sig := key.sign(key.kind, 'hello'.bytes()) or { panic(err) }
	host := parse_host_key(key.public_blob) or { panic(err) }
	host.verify_sig('ssh-ed25519', 'hello'.bytes(), sig) or { panic(err) }
	if _ := host.verify_sig('ssh-ed25519', 'hullo'.bytes(), sig) {
		assert false
	}
}

fn test_unsupported_and_garbage_keys_fail() {
	if _ := decode_private_key('not a key', none) {
		assert false
	}
	if _ := decode_private_key('-----BEGIN FOO-----\nAAAA\n-----END FOO-----\n', none) {
		assert false
	}
}
