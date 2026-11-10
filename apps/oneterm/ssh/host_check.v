module ssh

// HostCheck checks the server's host key against known_hosts; the zero value accepts any key
// (tests). Data, not a closure (no closures in apps, specs/14).
struct HostCheck {
	verify bool
	path   string
	host   string
	port   int
	policy HostKeyPolicy
}

fn (c HostCheck) check(key HostKey) ! {
	if c.verify {
		check_host_key(c.path, c.host, c.port, c.policy, key)!
	}
}
