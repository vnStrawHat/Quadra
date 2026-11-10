// JSON values and schema helpers shared by every OneTerm document.
module store

import x.json2

pub const schema_field = 'schema_version'

// parse_json decodes a document. x.json2 turns every number into f64, which would write integers
// back as `6.5536e+04`, so whole numbers are normalized to i64. Ceiling: above 2^53 they lose
// precision; no OneTerm document stores such a value.
pub fn parse_json(raw string) !json2.Any {
	return normalize(json2.decode[json2.Any](raw)!)
}

fn normalize(value json2.Any) json2.Any {
	match value {
		f64 {
			whole := i64(value)
			return if f64(whole) == value { json2.Any(whole) } else { value }
		}
		[]json2.Any {
			mut out := []json2.Any{cap: value.len}
			for item in value {
				out << normalize(item)
			}
			return json2.Any(out)
		}
		map[string]json2.Any {
			mut out := map[string]json2.Any{}
			for key, item in value {
				out[key] = normalize(item)
			}
			return json2.Any(out)
		}
		else {
			return value
		}
	}
}

// encode_pretty serializes with two-space indentation, like the Rust app.
pub fn encode_pretty(value json2.Any) string {
	return json2.encode(value, prettify: true, indent_string: '  ')
}

// object_of returns the fields of a JSON object, none for anything else.
pub fn object_of(value json2.Any) ?map[string]json2.Any {
	if value is map[string]json2.Any {
		return value
	}
	return none
}

// array_of returns the items of a JSON array, none for anything else.
pub fn array_of(value json2.Any) ?[]json2.Any {
	if value is []json2.Any {
		return value
	}
	return none
}

// string_of returns a JSON string, none for anything else.
pub fn string_of(value json2.Any) ?string {
	if value is string {
		return value
	}
	return none
}

// u64_of returns a non-negative JSON integer, none for anything else.
pub fn u64_of(value json2.Any) ?u64 {
	if value is i64 {
		if value >= 0 {
			return u64(value)
		}
	}
	return none
}

// bool_of returns a JSON boolean, none for anything else.
pub fn bool_of(value json2.Any) ?bool {
	if value is bool {
		return value
	}
	return none
}

// is_null reports a JSON null.
pub fn is_null(value json2.Any) bool {
	return value is json2.Null
}

// null_value is the JSON null, for documents that write one.
pub fn null_value() json2.Any {
	return json2.Any(json2.null)
}

// schema_version reads a document's version; an absent field is the original schema, 0.
pub fn schema_version(doc json2.Any) !u32 {
	fields := object_of(doc) or { return 0 }
	version := fields[schema_field] or { return 0 }
	number := u64_of(version) or { return error('schema version must be a non-negative integer') }
	if number > max_u32 {
		return error('schema version is too large')
	}
	return u32(number)
}

// set_schema_version stamps the current version on a JSON object.
pub fn set_schema_version(mut doc json2.Any, version u32) ! {
	mut fields := object_of(doc) or {
		return InvalidData{
			reason: 'versioned JSON document must be an object'
		}
	}
	fields[schema_field] = json2.Any(i64(version))
	doc = json2.Any(fields)
}

// versioned_object is an empty object carrying `version`.
pub fn versioned_object(version u32) json2.Any {
	mut fields := map[string]json2.Any{}
	fields[schema_field] = json2.Any(i64(version))
	return json2.Any(fields)
}

pub type MigrateStep = fn (version u32, value json2.Any) !json2.Any

// migrate_json_value applies sequential migrations; a newer document is refused so an older
// binary can never truncate it.
pub fn migrate_json_value(value_ json2.Any, current u32, name string, step MigrateStep) !json2.Any {
	mut value := value_
	mut version := schema_version(value) or { return InvalidData{
		reason: err.msg()
	} }
	if version > current {
		return InvalidData{
			reason: '${name} schema version ${version} is newer than supported version ${current}'
		}
	}
	for version < current {
		value = step(version, value)!
		version++
		set_schema_version(mut value, version)!
	}
	return value
}

// parse_versioned_object decodes a document that is a JSON object at every version: older
// documents read through their defaults, newer ones are refused.
pub fn parse_versioned_object(raw string, current u32, name string) !json2.Any {
	value := parse_json(raw) or { return InvalidData{
		reason: '${name}: ${err.msg()}'
	} }
	if object_of(value) == none {
		return InvalidData{
			reason: '${name} schema must be an object'
		}
	}
	return migrate_json_value(value, current, name, fn (_ u32, v json2.Any) !json2.Any {
		return v
	})
}
