// The SSH agent protocol (draft-miller-ssh-agent) and the forwarding bridge.
module ssh

const agent_failure = u8(5)
const agent_request_identities = u8(11)
const agent_identities_answer = u8(12)
const agent_sign_request = u8(13)
const agent_sign_response = u8(14)
const agent_flag_sha256 = u32(2)
const agent_flag_sha512 = u32(4)
const agent_max_message = 256 * 1024

// AgentLink is one connection to an agent: a request/response exchange of bare messages.
interface AgentLink {
mut:
	query(request []u8) ![]u8
	close()
}

struct Identity {
	blob    []u8
	comment string
}

fn agent_identities(mut link AgentLink) ![]Identity {
	reply := link.query([agent_request_identities])!
	if reply.len == 0 || reply[0] != agent_identities_answer {
		return error('unexpected agent reply')
	}
	mut r := new_reader(reply[1..])
	n := int(r.get_u32()!)
	mut out := []Identity{}
	for _ in 0 .. n {
		blob := r.get_bytes()!
		comment := r.get_str()!
		out << Identity{blob.clone(), comment}
	}
	return out
}

// agent_sign returns the wire signature blob for data.
fn agent_sign(mut link AgentLink, blob []u8, data []u8, flags u32) ![]u8 {
	mut w := new_writer(blob.len + data.len + 16)
	w.put_u8(agent_sign_request)
	w.put_bytes(blob)
	w.put_bytes(data)
	w.put_u32(flags)
	reply := link.query(w.buf)!
	if reply.len == 0 || reply[0] != agent_sign_response {
		return error('the agent refused to sign')
	}
	mut r := new_reader(reply[1..])
	return r.get_bytes()!.clone()
}

// bridge_agent carries one forwarded agent channel to a fresh local agent connection, message
// by message; it ends when either side does.
fn bridge_agent(mut ch Channel, pipe string) {
	mut link := connect_agent(pipe) or {
		ch.close()
		return
	}
	defer {
		link.close()
		ch.close()
	}
	mut header := []u8{len: 4}
	for {
		ch.read_full(mut header) or { return }
		n := int(u32(header[0]) << 24 | u32(header[1]) << 16 | u32(header[2]) << 8 | u32(header[3]))
		if n < 1 || n > agent_max_message {
			return
		}
		mut request := []u8{len: n}
		ch.read_full(mut request) or { return }
		reply := link.query(request) or { [agent_failure] }
		mut out := new_writer(reply.len + 4)
		out.put_bytes(reply)
		ch.write(out.buf) or { return }
	}
}
