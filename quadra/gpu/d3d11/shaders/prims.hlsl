// Uber-instance pipeline: every primitive kind in one instanced draw, in paint order.
// Must match gpu.Instance (quadra/gpu/primitives.v), 144 bytes, tightly packed.

struct Instance {
    float4 bounds;   // x, y, width, height (device px)
    float4 clip;     // content mask rect
    float4 color;    // straight alpha
    float4 color2;   // border / gradient end / shadow color
    float4 radii;    // tl, tr, br, bl
    float4 border;   // widths t, r, b, l (shadow: inset of the casting box)
    float4 uv;       // atlas u0, v0, u1, v1
    float4 params;   // kind specific; .w = clip corner radius
    uint kind;       // 0 quad, 1 shadow, 2 mono, 3 poly, 4 line
    uint page;
    uint flags;      // quad: 1 = gradient, 2 = border; mono: 1 = text glyph
    uint xform;      // own transform (low 16 bits) and the clip's (high): xforms indices
};

StructuredBuffer<Instance> instances : register(t0);
Texture2D<float> mono_atlas : register(t1);
Texture2D<float4> poly_atlas : register(t2);
SamplerState atlas_sampler : register(s0);
// Rigid transforms of rotated subtrees (cos, sin, tx, ty; device px); 0 is the identity.
StructuredBuffer<float4> xforms : register(t3);

float2 rigid(float4 r, float2 p) {
    return float2(r.x * p.x - r.y * p.y, r.y * p.x + r.x * p.y) + r.zw;
}

float2 unrigid(float4 r, float2 p) {
    float2 d = p - r.zw;
    return float2(r.x * d.x + r.y * d.y, r.x * d.y - r.y * d.x);
}

cbuffer Globals : register(b0) {
    float2 viewport;
    uint instance_offset;
    float globals_pad;
};

struct VsOut {
    float4 pos : SV_Position;
    float2 px : PIXEL;      // in the primitive's own space
    float2 screen : SCREEN; // after its transform
    float2 uv : UV;
    nointerpolation uint id : ID;
};

VsOut vs_main(uint vid : SV_VertexID, uint iid : SV_InstanceID) {
    uint id = iid + instance_offset;
    Instance p = instances[id];
    float2 corner = float2(vid & 1, vid >> 1);
    float2 px = p.bounds.xy + corner * p.bounds.zw;
    uint own = p.xform & 0xffff;
    float2 screen = own != 0 ? rigid(xforms[own], px) : px;
    VsOut o;
    o.pos = float4(screen / viewport * float2(2, -2) + float2(-1, 1), 0, 1);
    o.px = px;
    o.screen = screen;
    o.uv = lerp(p.uv.xy, p.uv.zw, corner);
    o.id = id;
    return o;
}

float sdf_rrect(float2 p, float2 center, float2 half_size, float4 radii) {
    float2 q = p - center;
    float r = q.x < 0 ? (q.y < 0 ? radii.x : radii.w) : (q.y < 0 ? radii.y : radii.z);
    r = min(r, min(half_size.x, half_size.y));
    float2 d = abs(q) - half_size + r;
    return length(max(d, 0)) + min(max(d.x, d.y), 0) - r;
}

float4 premul(float4 c, float coverage) {
    float a = c.a * coverage;
    return float4(c.rgb * a, a);
}

// DirectWrite's default gamma and enhanced contrast (checked against 1.0/0, 2.2 and 1.0 on
// the gallery: these keep light-on-dark text firm without bolding it).
#define TEXT_GAMMA 1.8
#define TEXT_CONTRAST 0.5

// Glyph coverage is a linear area fraction, but it is blended in a gamma-encoded target: as-is
// that thins light text on dark planes and thickens dark text on light ones. The background is
// unknown here, so the correction follows the text's own luminance: light text gets a gamma
// lift, all text a mild contrast boost that firms up stems without growing them.
// ClearType glyphs run it per subpixel.
float3 text_coverage(float3 a, float3 rgb) {
    float k = TEXT_CONTRAST;
    a = saturate(a * (1 + k) / (1 + k * a));
    float l = dot(rgb, float3(0.2126, 0.7152, 0.0722));
    return lerp(a, pow(a, 1.0 / TEXT_GAMMA), l);
}

// Abramowitz-Stegun erf approximation, good to ~1e-4: plenty for shadows.
float erf_approx(float x) {
    float s = sign(x);
    float a = abs(x);
    float t = 1.0 / (1.0 + 0.3275911 * a);
    float y = 1.0 - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t * exp(-a * a);
    return s * y;
}

float4 shade_quad(Instance p, float2 px) {
    float2 half_size = p.bounds.zw * 0.5;
    float2 center = p.bounds.xy + half_size;
    float coverage = saturate(0.5 - sdf_rrect(px, center, half_size, p.radii));
    float4 fill = p.color;
    if (p.flags & 1) {
        float ang = radians(p.params.x);
        float2 dir = float2(cos(ang), sin(ang));
        float t = dot((px - p.bounds.xy) / max(p.bounds.zw, 1) - 0.5, dir) + 0.5;
        fill = lerp(p.color, p.color2, saturate(t));
    }
    float4 c = premul(fill, 1);
    if (p.flags & 2) {
        float4 b = p.border; // t r b l
        // Sides without a border push the inner edge outside the box, so their
        // antialiasing band never leaks border color along that side.
        float4 none = float4(b.x <= 0, b.y <= 0, b.z <= 0, b.w <= 0) * 2;
        float2 inner_min = p.bounds.xy + float2(b.w, b.x) - float2(none.w, none.x);
        float2 inner_max = p.bounds.xy + p.bounds.zw - float2(b.y, b.z) + float2(none.y, none.z);
        float2 inner_half = max((inner_max - inner_min) * 0.5, 0);
        float4 inner_radii = max(p.radii - float4(max(b.w, b.x), max(b.y, b.x), max(b.y, b.z), max(b.w, b.z)), 0);
        float inside = saturate(0.5 - sdf_rrect(px, inner_min + inner_half, inner_half, inner_radii));
        c = lerp(premul(p.color2, 1), c, inside);
    }
    return c * coverage;
}

float4 shade_shadow(Instance p, float2 px) {
    float pad = p.border.x;
    float2 box_min = p.bounds.xy + pad;
    float2 half_size = max(p.bounds.zw * 0.5 - pad, 0);
    float d = sdf_rrect(px, box_min + half_size, half_size, p.radii);
    float sigma = max(p.params.x * 0.5, 0.5);
    float a = 0.5 - 0.5 * erf_approx(d / (sigma * 1.41421356));
    return premul(p.color2, a);
}

// column_ink is how much of a chart line's column lies above y: the line, tv tall
// vertically, travels d down the 1 px column, so its coverage is a trapezoid (two boxes
// convolved); this is its integral from the top (R2 is the ramp's antiderivative).
float r2(float x) {
    x = max(x, 0);
    return x * x * 0.5;
}

float column_ink(float y, float tv, float d) {
    return (r2(y) - r2(y - d) - r2(y - tv) + r2(y - tv - d)) / d;
}

// seg_dist is the distance from p to the segment a-b.
float seg_dist(float2 p, float2 a, float2 b) {
    float2 pa = p - a, ba = b - a;
    float h = saturate(dot(pa, ba) / max(dot(ba, ba), 1e-6));
    return length(pa - ba * h);
}

float4 shade_line(Instance p, float2 px) {
    if (p.params.y > 2.5) { // a stroke segment (gpu/stroke.v): radii = a, b; uv = prev, next
        float d = seg_dist(px, p.radii.xy, p.radii.zw);
        // The nearest segment draws a joint's pixel (ties go to the later one), so joints
        // are covered once.
        if ((p.flags & 1) && seg_dist(px, p.uv.xy, p.radii.xy) < d) return 0;
        if ((p.flags & 2) && seg_dist(px, p.radii.zw, p.uv.zw) <= d) return 0;
        return premul(p.color, saturate(p.params.x * 0.5 + 0.5 - d));
    }
    if (p.params.y > 1.5) { // a chart line's column (1 px padding above and below)
        float tv = p.params.x, d = max(p.params.z, 0.01);
        float y = px.y - p.bounds.y - 1;
        return premul(p.color, saturate(column_ink(y + 0.5, tv, d) - column_ink(y - 0.5, tv, d)));
    }
    float thickness = max(p.params.x, 1);
    if (p.params.y > 0.5) {
        float mid = p.bounds.y + p.bounds.w * 0.5;
        float amp = max(p.bounds.w * 0.5 - thickness * 0.5, 0.5);
        float phase = (px.x - p.bounds.x) * 0.7854;
        float wave = mid + sin(phase) * amp;
        // Distance across the wave, not straight down: even width on the slopes too.
        float slope = cos(phase) * amp * 0.7854;
        float d = abs(px.y - wave) / sqrt(1 + slope * slope);
        return premul(p.color, saturate(thickness * 0.5 + 0.5 - d));
    }
    return premul(p.color, 1);
}

// Dual-source output: color is premultiplied, weight is the per-channel coverage the
// destination is attenuated by (dest * (1 - weight)). Everything but ClearType text has the
// same weight in every channel, which is plain premultiplied-alpha blending.
struct PsOut {
    float4 color : SV_Target0;
    float4 weight : SV_Target1;
};

PsOut ps_main(VsOut i) {
    Instance p = instances[i.id];
    float4 c;
    float3 w = -1;
    if (p.kind == 0) {
        c = shade_quad(p, i.px);
    } else if (p.kind == 1) {
        c = shade_shadow(p, i.px);
    } else if (p.kind == 2) {
        float cov = mono_atlas.Sample(atlas_sampler, i.uv);
        c = premul(p.color, (p.flags & 1) ? text_coverage(cov.xxx, p.color.rgb).x : cov);
    } else if (p.kind == 3) {
        float2 half_size = p.bounds.zw * 0.5;
        float corner = saturate(0.5 - sdf_rrect(i.px, p.bounds.xy + half_size, half_size, p.radii));
        float4 t = poly_atlas.Sample(atlas_sampler, i.uv);
        if (p.flags & 1) { // ClearType text: rgb is coverage per subpixel (gray when rotated)
            float3 cov = (p.xform & 0xffff) != 0 ? dot(t.rgb, 1.0 / 3).xxx : t.rgb;
            w = text_coverage(cov, p.color.rgb) * p.color.a;
            c = float4(p.color.rgb * w, max(w.r, max(w.g, w.b)));
        } else {
            c = t * p.color.a * corner;
        }
    } else {
        c = shade_line(p, i.px);
    }
    uint own = p.xform & 0xffff, cl = p.xform >> 16;
    float2 cp = cl == own ? i.px : (cl == 0 ? i.screen : unrigid(xforms[cl], i.screen));
    float2 clip_half = p.clip.zw * 0.5;
    float clip = saturate(0.5 - sdf_rrect(cp, p.clip.xy + clip_half, clip_half, p.params.wwww));
    PsOut o;
    o.color = c * clip;
    o.weight = float4(w.r < 0 ? o.color.aaa : w * clip, o.color.a);
    return o;
}
