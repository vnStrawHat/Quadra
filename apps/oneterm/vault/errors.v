// Typed vault errors. None of them carries secret material.
module vault

pub enum VaultErrorKind {
	wrong_password      // the master password does not unwrap the data key
	empty_password      // an empty master password is never accepted
	already_initialised // set-up found a vault.json already there
	vault_replaced      // vault.json was reset or replaced by another OneTerm
	changed_elsewhere   // same vault, password changed by another OneTerm
	read_only           // a read-only (elevated) window writes nothing
	secret_too_long     // the secret is longer than its purpose allows
	randomness          // the operating system's random number generator failed
	unsupported         // header or key derivation outside what this binary supports
	io                  // reading or writing vault.json failed
}

// VaultError says why a vault operation failed.
pub struct VaultError {
	Error
pub:
	kind   VaultErrorKind
	detail string
}

pub fn (e VaultError) msg() string {
	return match e.kind {
		.wrong_password { 'wrong master password' }
		.empty_password { 'the master password is empty' }
		.already_initialised { 'a master password is already set up' }
		.vault_replaced { 'the master password was reset or replaced in another OneTerm window' }
		.changed_elsewhere { 'the master password was changed in another OneTerm window' }
		.read_only { 'an elevated OneTerm window cannot change the master password' }
		.secret_too_long { 'the secret is too long to seal' }
		.randomness { "the operating system's random number generator failed" }
		.unsupported { 'unsupported vault: ${e.detail}' }
		.io { 'vault.json: ${e.detail}' }
	}
}

fn io_error(err IError) VaultError {
	if err is VaultError {
		return err
	}
	return VaultError{
		kind:   .io
		detail: err.msg()
	}
}

pub enum OpenErrorKind {
	other_vault  // sealed under another vault: the master password was reset since
	newer_format // written by a newer OneTerm (`otvN:`, N > 1); keep it untouched
	malformed    // not an envelope this binary can parse, including plain text
	rejected     // authentication failed: tampered, or sealed for another purpose or address
}

// OpenError says why a sealed value cannot be opened. It is never used as a password.
pub struct OpenError {
	Error
pub:
	kind OpenErrorKind
}

pub fn (e OpenError) msg() string {
	return match e.kind {
		.other_vault { 'saved under a master password that was reset' }
		.newer_format { 'saved by a newer OneTerm' }
		.malformed { 'not a sealed value' }
		.rejected { 'cannot be used for this address' }
	}
}
