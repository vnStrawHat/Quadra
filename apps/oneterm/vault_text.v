// The master-password dialogs' copy (specs/22, specs/25), as in the original.
module main

enum DialogKind {
	set_up
	unlock
	change
	reset
}

struct FieldInfo {
	label  string
	masked bool
}

fn (k DialogKind) title() string {
	return match k {
		.set_up { 'Create a master password' }
		.unlock { 'Unlock saved passwords' }
		.change { 'Change master password' }
		.reset { 'Reset master password' }
	}
}

fn (k DialogKind) confirm_label() string {
	return match k {
		.set_up { 'Create' }
		.unlock { 'Unlock' }
		.change { 'Change' }
		.reset { 'Reset' }
	}
}

fn (k DialogKind) intro() string {
	return match k {
		.set_up { 'OneTerm encrypts the passwords you save with it. Choose a long one: anyone with a copy of your OneTerm folder can try to guess it.' }
		.unlock { 'Enter your master password to use your saved passwords.' }
		.change { 'Saved passwords stay valid; only the master password changes.' }
		.reset { 'Reset deletes the master password and every saved password. Your sessions are kept. This cannot be undone.' }
	}
}

fn (k DialogKind) fields() []FieldInfo {
	master := FieldInfo{'Master password', true}
	mut out := []FieldInfo{}
	match k {
		// a match arm with an array literal generates broken C
		.set_up {
			out << master
			out << FieldInfo{'Confirm master password', true}
		}
		.unlock {
			out << master
		}
		.change {
			out << FieldInfo{'Current master password', true}
			out << FieldInfo{'New master password', true}
			out << FieldInfo{'Confirm new master password', true}
		}
		.reset {
			out << FieldInfo{'Type RESET to confirm', false}
		}
	}

	return out
}

fn (k DialogKind) new_password_field() int {
	return match k {
		.set_up { 0 }
		.change { 1 }
		else { -1 }
	}
}
