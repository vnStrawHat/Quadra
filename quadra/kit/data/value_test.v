module data

const manifest = '# A deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
  labels: {app: web, tier: "front end"}
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: web
          image: registry.local/web:1.4.2
          args: [--port, 8080, --verbose]
          env:
            - {name: DEBUG, value: "false"}
          command: &cmd
            - /bin/sh
          script: |
            echo starting # still part of the script

            exec web
      restartPolicy: Always
      tolerations:
      - key: a
        effect: NoSchedule
---
kind: Ignored
'

fn test_yaml_subset() {
	v := parse_yaml(manifest)!
	assert v.kind == .map
	assert v.keys == ['apiVersion', 'kind', 'metadata', 'spec']
	assert (v.at('.metadata.labels.tier')?).text == 'front end'
	r := v.at('.spec.replicas')?
	assert r.kind == .number && r.text == '3'
	c := v.at('.spec.template.spec.containers[0]')?
	assert c.keys == ['name', 'image', 'args', 'env', 'command', 'script']
	assert (c.get('image')?).text == 'registry.local/web:1.4.2'
	args := c.get('args')?
	assert args.items.map(it.text) == ['--port', '8080', '--verbose']
	assert args.items[1].kind == .number
	env := v.at('.spec.template.spec.containers[0].env[0].value')?
	assert env.kind == .str && env.text == 'false' // quoted: a string, not a bool
	assert (v.at('.spec.template.spec.containers[0].command[0]')?).text == '/bin/sh' // anchor skipped
	assert (c.get('script')?).text == 'echo starting # still part of the script\n\nexec web'
	assert (v.at('.spec.template.spec.restartPolicy')?).text == 'Always'
	// A sequence at the same indentation as its key.
	assert (v.at('.spec.template.spec.tolerations[0].effect')?).text == 'NoSchedule'
	assert v.get('kind')?.text == 'Deployment' // the second document is ignored
}

fn test_yaml_scalars_and_errors() {
	v := parse_yaml("a: ~\nb: true\nc: -1.5e3\nd: 'it''s'\ne: 0x1F\nf: 1.2.3\n")!
	assert v.items.map(it.kind) == [ValueKind.null, .boolean, .number, .str, .number, .str]
	assert v.items[3].text == "it's"
	if _ := parse_yaml('a: 1\n    b: 2\n') {
		assert false, 'bad indentation must fail'
	}
	empty := parse_yaml('# nothing\n')!
	assert empty.kind == .null
}

fn test_json() {
	v := parse_json('{"a": [1, 2.5, "x", null, true], "b": {"c": {}}}')!
	assert v.keys == ['a', 'b']
	a := v.get('a')?
	assert a.items.map(it.kind) == [ValueKind.number, .number, .str, .null, .boolean]
	assert a.summary() == '[5]'
	assert (v.at('.b.c')?).summary() == '{0}'
	if _ := parse_json('{"a":') {
		assert false
	}
}
