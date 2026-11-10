// What the panel asks the app to do and what it tells the app (specs/26). The credentials
// live only in these values; nothing here is logged or persisted.
module sessionui

import store

// Credential is the method and secrets for one connection hop. `str` never prints a secret.
pub struct Credential {
pub:
	method     store.AuthMethod
	password   string // empty for no password; never saved in plain form
	key_path   string
	passphrase string
}

pub fn (c Credential) str() string {
	return 'Credential{method: ${c.method}, key_path: "${c.key_path}", secrets redacted}'
}

// HopRequest is one jump host of a ConnectRequest, outermost first.
pub struct HopRequest {
pub:
	label              string
	host               string
	port               int
	username           string
	credential         Credential
	accept_fingerprint string
}

// ConnectRequest is emitted when the user connects. `attempt` identifies it in the callbacks;
// a retry after Trust and Connect keeps the id and sets `accept_fingerprint`.
pub struct ConnectRequest {
pub:
	attempt            u64
	session_id         ?u64 // none for Quick Connect and Duplicate
	label              string
	host               string
	port               int
	username           string
	credential         Credential
	hops               []HopRequest
	forwards           []store.PortForward
	agent_forwarding   bool
	logging            store.LoggingOverride
	shell_integration  bool = true
	initial_cwd        string
	tag                string
	accept_fingerprint string
}

// DuplicateSpec prefills the Duplicate SSH Session dialog from a running session; `tag` is
// returned unchanged in the ConnectRequest so the app can place the new tab.
pub struct DuplicateSpec {
pub:
	label             string
	host              string
	port              int
	username          string
	method            store.AuthMethod
	key_path          string
	hops              []HopSpec
	forwards          []store.PortForward
	agent_forwarding  bool
	shell_integration bool = true
	initial_cwd       string
	tag               string
}
