// TeX command names mapped to Unicode, with their atom class (TeX spacing, specs/21).
module tex

// Cls is a TeX atom class: it decides the space between neighbouring atoms.
pub enum Cls {
	ord
	op
	bin
	rel
	open
	close
	punct
	inner
}

struct Sym {
	ch  string
	cls Cls
}

const symbols = {
	// Greek (lowercase becomes italic in layout, as TeX sets it)
	'alpha':             Sym{'α', .ord}
	'beta':              Sym{'β', .ord}
	'gamma':             Sym{'γ', .ord}
	'delta':             Sym{'δ', .ord}
	'epsilon':           Sym{'ϵ', .ord}
	'varepsilon':        Sym{'ε', .ord}
	'zeta':              Sym{'ζ', .ord}
	'eta':               Sym{'η', .ord}
	'theta':             Sym{'θ', .ord}
	'vartheta':          Sym{'ϑ', .ord}
	'iota':              Sym{'ι', .ord}
	'kappa':             Sym{'κ', .ord}
	'lambda':            Sym{'λ', .ord}
	'mu':                Sym{'μ', .ord}
	'nu':                Sym{'ν', .ord}
	'xi':                Sym{'ξ', .ord}
	'omicron':           Sym{'ο', .ord}
	'pi':                Sym{'π', .ord}
	'varpi':             Sym{'ϖ', .ord}
	'rho':               Sym{'ρ', .ord}
	'varrho':            Sym{'ϱ', .ord}
	'sigma':             Sym{'σ', .ord}
	'varsigma':          Sym{'ς', .ord}
	'tau':               Sym{'τ', .ord}
	'upsilon':           Sym{'υ', .ord}
	'phi':               Sym{'ϕ', .ord}
	'varphi':            Sym{'φ', .ord}
	'chi':               Sym{'χ', .ord}
	'psi':               Sym{'ψ', .ord}
	'omega':             Sym{'ω', .ord}
	'Gamma':             Sym{'Γ', .ord}
	'Delta':             Sym{'Δ', .ord}
	'Theta':             Sym{'Θ', .ord}
	'Lambda':            Sym{'Λ', .ord}
	'Xi':                Sym{'Ξ', .ord}
	'Pi':                Sym{'Π', .ord}
	'Sigma':             Sym{'Σ', .ord}
	'Upsilon':           Sym{'Υ', .ord}
	'Phi':               Sym{'Φ', .ord}
	'Psi':               Sym{'Ψ', .ord}
	'Omega':             Sym{'Ω', .ord}
	// ordinary symbols
	'infty':             Sym{'∞', .ord}
	'partial':           Sym{'∂', .ord}
	'nabla':             Sym{'∇', .ord}
	'forall':            Sym{'∀', .ord}
	'exists':            Sym{'∃', .ord}
	'nexists':           Sym{'∄', .ord}
	'emptyset':          Sym{'∅', .ord}
	'varnothing':        Sym{'∅', .ord}
	'neg':               Sym{'¬', .ord}
	'lnot':              Sym{'¬', .ord}
	'hbar':              Sym{'ℏ', .ord}
	'ell':               Sym{'ℓ', .ord}
	'Re':                Sym{'ℜ', .ord}
	'Im':                Sym{'ℑ', .ord}
	'aleph':             Sym{'ℵ', .ord}
	'wp':                Sym{'℘', .ord}
	'angle':             Sym{'∠', .ord}
	'triangle':          Sym{'△', .ord}
	'prime':             Sym{'′', .ord}
	'degree':            Sym{'°', .ord}
	'top':               Sym{'⊤', .ord}
	'bot':               Sym{'⊥', .ord}
	'ldots':             Sym{'…', .inner}
	'dots':              Sym{'…', .inner}
	'cdots':             Sym{'⋯', .inner}
	'vdots':             Sym{'⋮', .ord}
	'ddots':             Sym{'⋱', .inner}
	'{':                 Sym{'{', .open}
	'}':                 Sym{'}', .close}
	'|':                 Sym{'‖', .ord}
	'%':                 Sym{'%', .ord}
	'$':                 Sym{'$', .ord}
	'#':                 Sym{'#', .ord}
	'&':                 Sym{'&', .ord}
	'_':                 Sym{'_', .ord}
	'langle':            Sym{'⟨', .open}
	'rangle':            Sym{'⟩', .close}
	'lfloor':            Sym{'⌊', .open}
	'rfloor':            Sym{'⌋', .close}
	'lceil':             Sym{'⌈', .open}
	'rceil':             Sym{'⌉', .close}
	'lvert':             Sym{'|', .open}
	'rvert':             Sym{'|', .close}
	'lVert':             Sym{'‖', .open}
	'rVert':             Sym{'‖', .close}
	'vert':              Sym{'|', .ord}
	'Vert':              Sym{'‖', .ord}
	'lbrace':            Sym{'{', .open}
	'rbrace':            Sym{'}', .close}
	// binary operators
	'pm':                Sym{'±', .bin}
	'mp':                Sym{'∓', .bin}
	'times':             Sym{'×', .bin}
	'div':               Sym{'÷', .bin}
	'cdot':              Sym{'⋅', .bin}
	'ast':               Sym{'∗', .bin}
	'star':              Sym{'⋆', .bin}
	'circ':              Sym{'∘', .bin}
	'bullet':            Sym{'∙', .bin}
	'oplus':             Sym{'⊕', .bin}
	'ominus':            Sym{'⊖', .bin}
	'otimes':            Sym{'⊗', .bin}
	'odot':              Sym{'⊙', .bin}
	'cap':               Sym{'∩', .bin}
	'cup':               Sym{'∪', .bin}
	'wedge':             Sym{'∧', .bin}
	'land':              Sym{'∧', .bin}
	'vee':               Sym{'∨', .bin}
	'lor':               Sym{'∨', .bin}
	'setminus':          Sym{'∖', .bin}
	'bmod':              Sym{'mod', .bin}
	// relations
	'leq':               Sym{'≤', .rel}
	'le':                Sym{'≤', .rel}
	'geq':               Sym{'≥', .rel}
	'ge':                Sym{'≥', .rel}
	'neq':               Sym{'≠', .rel}
	'ne':                Sym{'≠', .rel}
	'equiv':             Sym{'≡', .rel}
	'approx':            Sym{'≈', .rel}
	'sim':               Sym{'∼', .rel}
	'simeq':             Sym{'≃', .rel}
	'cong':              Sym{'≅', .rel}
	'propto':            Sym{'∝', .rel}
	'll':                Sym{'≪', .rel}
	'gg':                Sym{'≫', .rel}
	'prec':              Sym{'≺', .rel}
	'succ':              Sym{'≻', .rel}
	'in':                Sym{'∈', .rel}
	'notin':             Sym{'∉', .rel}
	'ni':                Sym{'∋', .rel}
	'subset':            Sym{'⊂', .rel}
	'supset':            Sym{'⊃', .rel}
	'subseteq':          Sym{'⊆', .rel}
	'supseteq':          Sym{'⊇', .rel}
	'perp':              Sym{'⊥', .rel}
	'parallel':          Sym{'∥', .rel}
	'mid':               Sym{'∣', .rel}
	'models':            Sym{'⊨', .rel}
	'vdash':             Sym{'⊢', .rel}
	'coloneqq':          Sym{'≔', .rel}
	'doteq':             Sym{'≐', .rel}
	// arrows
	'to':                Sym{'→', .rel}
	'rightarrow':        Sym{'→', .rel}
	'leftarrow':         Sym{'←', .rel}
	'gets':              Sym{'←', .rel}
	'leftrightarrow':    Sym{'↔', .rel}
	'Rightarrow':        Sym{'⇒', .rel}
	'Leftarrow':         Sym{'⇐', .rel}
	'Leftrightarrow':    Sym{'⇔', .rel}
	'implies':           Sym{'⟹', .rel}
	'impliedby':         Sym{'⟸', .rel}
	'iff':               Sym{'⟺', .rel}
	'mapsto':            Sym{'↦', .rel}
	'longrightarrow':    Sym{'⟶', .rel}
	'longleftarrow':     Sym{'⟵', .rel}
	'Longrightarrow':    Sym{'⟹', .rel}
	'uparrow':           Sym{'↑', .rel}
	'downarrow':         Sym{'↓', .rel}
	'nearrow':           Sym{'↗', .rel}
	'searrow':           Sym{'↘', .rel}
	'hookrightarrow':    Sym{'↪', .rel}
	'rightleftharpoons': Sym{'⇌', .rel}
	// punctuation
	'colon':             Sym{':', .punct}
}

// Big operators: their glyph and whether limits go above and below in display style.
struct BigOp {
	ch     string
	limits bool
}

const big_ops = {
	'sum':       BigOp{'∑', true}
	'prod':      BigOp{'∏', true}
	'coprod':    BigOp{'∐', true}
	'bigcup':    BigOp{'⋃', true}
	'bigcap':    BigOp{'⋂', true}
	'bigvee':    BigOp{'⋁', true}
	'bigwedge':  BigOp{'⋀', true}
	'bigoplus':  BigOp{'⨁', true}
	'bigotimes': BigOp{'⨂', true}
	'int':       BigOp{'∫', false}
	'iint':      BigOp{'∬', false}
	'iiint':     BigOp{'∭', false}
	'oint':      BigOp{'∮', false}
}

// Function names set upright; true = limits below in display (\lim_{x\to0}).
const functions = {
	'sin':    false
	'cos':    false
	'tan':    false
	'cot':    false
	'sec':    false
	'csc':    false
	'arcsin': false
	'arccos': false
	'arctan': false
	'sinh':   false
	'cosh':   false
	'tanh':   false
	'coth':   false
	'log':    false
	'ln':     false
	'lg':     false
	'exp':    false
	'dim':    false
	'ker':    false
	'deg':    false
	'arg':    false
	'hom':    false
	'det':    true
	'gcd':    true
	'Pr':     true
	'lim':    true
	'liminf': true
	'limsup': true
	'max':    true
	'min':    true
	'sup':    true
	'inf':    true
}
