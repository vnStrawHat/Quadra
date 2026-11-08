// Fractions, radicals, scripts, limits, big operators and accents.
module tex

import quadra.prim

const numerator_style = [1, 2, 3, 3]

fn (mut l Lay) frac(n Node, st0 int) Box {
	st := if n.level >= 0 { n.level } else { st0 }
	inner := numerator_style[st]
	num := l.list(n.a, inner)
	den := l.list(n.b, inner)
	em := l.em(st)
	th := if n.rule { l.rule_w(st) } else { f32(0) }
	ax := l.axis(st)
	mut u := if st == 0 {
		em * 0.677
	} else if n.rule {
		em * 0.394
	} else {
		em * 0.444
	}
	mut v := if st == 0 { em * 0.686 } else { em * 0.345 }
	if n.rule {
		gap := if st == 0 { 3 * th } else { th }
		u = prim.max(u, ax + th / 2 + gap + num.d)
		v = prim.max(v, den.h - ax + th / 2 + gap)
	} else {
		gap := if st == 0 { 7 * l.rule_w(st) } else { 3 * l.rule_w(st) }
		short := gap - ((u - num.d) - (den.h - v))
		if short > 0 {
			u += short / 2
			v += short / 2
		}
	}
	pad := em * 0.12
	w := prim.max(num.w, den.w) + 2 * pad
	mut b := Box{
		w: w
		h: u + num.h
		d: v + den.d
	}
	b.put(num, (w - num.w) / 2, -u)
	b.put(den, (w - den.w) / 2, v)
	if n.rule {
		b.rule(pad * 0.5, -(ax + th / 2), w - pad, th)
	}
	return b
}

fn (mut l Lay) sqrt(n Node, st int) Box {
	x := l.list(n.body, st)
	em := l.em(st)
	th := l.rule_w(st)
	clear := if st == 0 { th + em * 0.11 } else { prim.max(th * 1.25, em * 0.08) }
	top := prim.max(x.h, em * 0.45) + clear + th
	bot := prim.max(x.d, em * 0.05) + em * 0.04
	total := top + bot
	sign_w := prim.min(em * 0.55 + total * 0.05, em * 1.1)
	mut kern := f32(0)
	mut idx := Box{}
	if n.has_a {
		idx = l.list(n.a, 3)
		kern = prim.max(0, idx.w - sign_w * 0.45)
	}
	tick := bot - prim.min(em * 0.5, total * 0.45)
	p0 := prim.pt(kern + em * 0.03, tick + em * 0.07)
	p1 := prim.pt(kern + em * 0.15, tick)
	p2 := prim.pt(kern + sign_w * 0.5, bot)
	p3 := prim.pt(kern + sign_w, -top + th / 2)
	p4 := prim.pt(kern + sign_w + x.w + em * 0.08, -top + th / 2)
	mut b := Box{
		w: p4.x + em * 0.04
		h: top
		d: bot
	}
	b.stroke([p0, p1], th)
	b.stroke([p1, p2], th * 2)
	b.stroke([p2, p3, p4], th)
	b.put(x, kern + sign_w + em * 0.04, 0)
	if n.has_a {
		y := bot - total * 0.6 - idx.d
		b.put(idx, kern + sign_w * 0.45 - idx.w, y)
		b.grow(idx, y)
	}
	return b
}

fn uses_limits(n Node, st int) bool {
	return n.kind in [.bigop, .fn_name] && (n.limits == 1 || (n.limits == 0 && st == 0))
}

fn (mut l Lay) scripts(n Node, st int) Box {
	base_node := n.body[0]
	if uses_limits(base_node, st) {
		return l.limits(l.node(base_node, st), n, st)
	}
	base := l.node(base_node, st)
	ss := script_style(st)
	em := l.em(st)
	sem := l.em(ss)
	xh := em * 0.45
	sup := if n.has_a { l.list(n.a, ss) } else { Box{} }
	sub := if n.has_b { l.list(n.b, ss) } else { Box{} }
	simple := base_node.kind == .atom
	mut u := if simple { f32(0) } else { base.h - sem * 0.386 }
	mut v := if simple { f32(0) } else { base.d + sem * 0.05 }
	if n.has_a {
		p := if st == 0 { em * 0.413 } else { em * 0.363 }
		u = prim.max(prim.max(u, p), sup.d + xh / 4)
	}
	if n.has_b && !n.has_a {
		v = prim.max(prim.max(v, em * 0.15), sub.h - xh * 0.8)
	}
	if n.has_b && n.has_a {
		v = prim.max(v, em * 0.247)
		need := 4 * l.rule_w(st) - ((u - sup.d) - (sub.h - v))
		if need > 0 {
			v += need
		}
	}
	mut b := Box{
		w: base.w
		h: base.h
		d: base.d
	}
	b.put(base, 0, 0)
	mut right := base.w
	if n.has_a {
		b.put(sup, base.w + base.ic, -u)
		b.grow(sup, -u)
		right = prim.max(right, base.w + base.ic + sup.w)
	}
	if n.has_b {
		b.put(sub, base.w, v)
		b.grow(sub, v)
		right = prim.max(right, base.w + sub.w)
	}
	b.w = right + em * 0.05
	return b
}

// limits stacks n.a over base and n.b under it, centered (display \sum, \lim, \overset).
fn (mut l Lay) limits(base Box, n Node, st int) Box {
	ss := script_style(st)
	em := l.em(st)
	over := if n.has_a { l.list(n.a, ss) } else { Box{} }
	under := if n.has_b { l.list(n.b, ss) } else { Box{} }
	gap := em * 0.12
	w := prim.max(prim.max(base.w, over.w), under.w)
	mut b := Box{
		w: w
		h: base.h
		d: base.d
	}
	b.put(base, (w - base.w) / 2, 0)
	if n.has_a {
		y := -(base.h + gap + over.d)
		b.put(over, (w - over.w) / 2, y)
		b.grow(over, y)
	}
	if n.has_b {
		y := base.d + gap + under.h
		b.put(under, (w - under.w) / 2, y)
		b.grow(under, y)
	}
	return b
}

fn (mut l Lay) bigop(n Node, st int) Box {
	integral := n.text in ['∫', '∬', '∭', '∮']
	scale := if st == 0 {
		if integral { f32(2.0) } else { 1.8 }
	} else {
		if integral { f32(1.3) } else { 1.1 }
	}
	g := l.glyph_at(n.text, l.em(st) * scale)
	// Centered on the math axis.
	shift := (g.h - g.d) / 2 - l.axis(st)
	mut b := Box{
		w:  g.w + l.em(st) * 0.05
		h:  g.h - shift
		d:  g.d + shift
		ic: g.ic
	}
	b.put(g, 0, shift)
	return b
}

fn (mut l Lay) accent(n Node, st int) Box {
	body := l.list(n.body, st)
	em := l.em(st)
	if n.text == '' { // \overline, \underline: a rule
		th := l.rule_w(st)
		mut b := Box{
			w: body.w
			h: body.h
			d: body.d
		}
		b.put(body, 0, 0)
		if n.under {
			b.rule(0, body.d + 2 * th, body.w, th)
			b.d = body.d + 4 * th
		} else {
			b.rule(0, -(body.h + 3 * th), body.w, th)
			b.h = body.h + 4 * th
		}
		return b
	}
	mark := l.glyph_at(n.text, if n.text == '→' { em * 0.7 } else { em })
	gap := em * 0.06
	lo := if mark.lo > -1e8 { mark.lo } else { f32(0) }
	y := -(prim.max(body.h, em * 0.45) + gap) - lo
	mut b := Box{
		w: prim.max(body.w, mark.w)
		h: -y + mark.h
		d: body.d
	}
	b.put(body, (b.w - body.w) / 2, 0)
	b.put(mark, (b.w - mark.w) / 2 + body.ic / 2, y)
	return b
}
