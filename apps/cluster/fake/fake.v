// A deterministic fake cluster: no network, the same resources on every run, so the app
// and its tests behave the same everywhere. live.v makes it change over time.
module fake

pub struct Resource {
pub mut:
	kind      string
	name      string
	namespace string
	status    string // Running, Pending, Failed, Succeeded, Terminating, Unknown
	ready     string // e.g. 1/1
	restarts  int
	age_s     int
	node      string
	image     string
	replicas  int    // deployments
	app       string // the app the resource belongs to
}

// key identifies a resource within its kind.
pub fn (r &Resource) key() string {
	return '${r.namespace}/${r.name}'
}

pub struct Event {
pub:
	id      int
	typ     string // Normal or Warning
	reason  string
	object  string // Kind/name
	message string
	at      int // cluster second it happened
}

pub struct LogLine {
pub:
	at_ms i64 // unix ms
	text  string
}

@[heap]
pub struct Cluster {
pub mut:
	name      string = 'kind-dev'
	resources []Resource
	t         int                  // seconds since the cluster started
	cpu       map[string][]f64     // millicores per pod key, oldest first, history_len samples
	mem       map[string][]f64     // MiB
	logs      map[string][]LogLine // the last max_logs lines per pod key
	written   map[string]int       // lines ever written per pod key (consumers track this)
	events    []Event
	next      int // counter for new pod names and event ids
}

pub const history_len = 120
pub const max_logs = 2000
pub const max_events = 500
pub const start_ms = i64(1793966400000) // 2026-11-07T12:00:00Z

const namespaces = ['default', 'kube-system', 'monitoring', 'ingress-nginx']
const apps = ['api', 'worker', 'web', 'redis', 'postgres', 'prometheus', 'grafana', 'coredns',
	'ingress-controller', 'cron-backup']
const statuses = ['Running', 'Running', 'Running', 'Running', 'Running', 'Pending', 'Running',
	'Failed', 'Running', 'Succeeded', 'Running', 'Terminating']

// suffix is a stable pod-name suffix for index i.
fn suffix(i int) string {
	alphabet := 'bcdfghjklmnpqrstvwxz2456789'
	mut s := []u8{}
	mut v := u32(i) * 2654435761
	for _ in 0 .. 5 {
		s << alphabet[int(v % u32(alphabet.len))]
		v /= u32(alphabet.len)
	}
	return s.bytestr()
}

fn new_pod(i int, app string, status string) Resource {
	return Resource{
		app:       app
		kind:      'Pod'
		name:      '${app}-7d9f8c6b5-${suffix(i)}'
		namespace: namespaces[(i / apps.len) % namespaces.len]
		status:    status
		ready:     if status == 'Running' { '1/1' } else { '0/1' }
		node:      'pool-a-${i % 4 + 1}'
		image:     '${app}:1.${i % 5}.${i % 3}'
	}
}

pub fn new_cluster() &Cluster {
	mut rs := []Resource{}
	for i in 0 .. 124 {
		mut p := new_pod(i, apps[i % apps.len], statuses[i % statuses.len])
		p.restarts = if i % 9 == 0 { i % 7 } else { 0 }
		p.age_s = (i * 3607) % 864000 + 60
		rs << p
	}
	for i, app in apps {
		rs << Resource{
			kind:      'Deployment'
			app:       app
			name:      app
			namespace: namespaces[i % namespaces.len]
			status:    'Running'
			ready:     '${i % 3 + 1}/${i % 3 + 1}'
			age_s:     (i + 1) * 86400
			image:     '${app}:1.${i % 5}.0'
			replicas:  i % 3 + 1
		}
		rs << Resource{
			kind:      'Service'
			app:       app
			name:      app
			namespace: namespaces[i % namespaces.len]
			status:    'Running'
			age_s:     (i + 1) * 86400
		}
		rs << Resource{
			kind:      'ConfigMap'
			app:       app
			name:      '${app}-config'
			namespace: namespaces[i % namespaces.len]
			status:    'Unknown'
			age_s:     (i + 2) * 86400
		}
	}
	mut c := &Cluster{
		resources: rs
		next:      124
	}
	c.warm_up()
	return c
}

pub fn (c &Cluster) of(kind string) []Resource {
	return c.resources.filter(it.kind == kind)
}

// find returns the resource of a kind by key.
pub fn (c &Cluster) find(kind string, key string) ?Resource {
	for r in c.resources {
		if r.kind == kind && r.key() == key {
			return r
		}
	}
	return none
}

// age formats seconds the way cluster tools do: 45s, 12m, 3h, 6d.
pub fn age(s int) string {
	return if s < 60 {
		'${s}s'
	} else if s < 3600 {
		'${s / 60}m'
	} else if s < 86400 {
		'${s / 3600}h'
	} else {
		'${s / 86400}d'
	}
}

// yaml is a short manifest for a resource.
pub fn (r Resource) yaml() string {
	mut s := 'apiVersion: ${if r.kind == 'Deployment' { 'apps/v1' } else { 'v1' }}\nkind: ${r.kind}\nmetadata:\n  name: ${r.name}\n  namespace: ${r.namespace}\n  labels:\n    app: ${r.app}\n'
	if r.kind == 'Deployment' {
		s += 'spec:\n  replicas: ${r.replicas}\n  template:\n    spec:\n      containers:\n        - name: main\n          image: ${r.image}\n'
	} else if r.image != '' {
		s += 'spec:\n  containers:\n    - name: main\n      image: ${r.image}\n      ports: [8080]\n'
	}
	if r.kind == 'Pod' {
		s += 'status:\n  phase: ${r.status}\n  hostIP: 10.0.0.${r.node.len}\n  restartCount: ${r.restarts}\n'
	}
	return s
}
