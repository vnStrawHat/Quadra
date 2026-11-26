// One pipeline for every primitive: quads, shadows, glyphs, images and lines are instances of
// one format, drawn in paint order. Must match quadra_render::Instance: 144 bytes.
//
// An instance is in its layer's space, in device pixels. The layer's transform and opacity
// come from the constants, which are set for every run (specs/07, "Layers").
//
// The Windows renderer's shader (quadra-native-win/shaders/prims.hlsl) in Metal's language:
// the two say the same, line for line, but for what is noted at `fs_main`.

#include <metal_stdlib>
using namespace metal;

struct Instance {
    float4 bounds;   // x, y, width, height
    float4 clip;     // clip rect, in the same space
    float4 color;    // straight alpha
    float4 color2;   // border / shadow color
    float4 radii;    // tl, tr, br, bl
    float4 border;   // widths t, r, b, l (shadow: inset of the casting box)
    float4 uv;       // atlas u0, v0, u1, v1 (a gradient's ramp)
    float4 params;   // kind specific; .w = clip corner radius
    uint kind;       // 0 quad, 1 shadow, 2 mono, 3 poly, 4 line (5 marks a child layer: never drawn)
    uint slice;      // slice of the atlas texture array
    uint flags;      // quad: 1 = gradient, 2 = border, 4 = dashed, 8 = dotted, 16 = backdrop; mono and poly: 1 = text glyph
    uint reserved;
};

struct Constants {
    float4 layer_matrix;    // a, b, c, d: x' = a*x + c*y + tx, y' = b*x + d*y + ty
    float2 layer_translate; // tx, ty
    float2 viewport;        // of the surface, in device pixels
    float layer_opacity;
    uint instance_offset;   // the first instance of the run
    uint layer_turned;      // 1 when the transform is more than a translation
    uint constants_pad;
    float4 backdrop_rect;   // where the backdrop was copied from on the surface; its texture's size
    float4 mask_rect;       // the box of the mask the layer is cut to, on the surface
    float4 mask_uv;         // its tile in the coverage atlas
    uint mask_slice;
    uint mask_on;
    uint2 mask_pad;
};

struct VsOut {
    float4 pos [[position]];
    float2 px; // in the layer's space
    float2 uv;
    uint id [[flat]];
};

vertex VsOut vs_main(uint vid [[vertex_id]], uint iid [[instance_id]],
                     const device Instance *instances [[buffer(0)]],
                     constant Constants &k [[buffer(1)]]) {
    uint id = iid + k.instance_offset;
    Instance p = instances[id];
    float2 corner = float2(float(vid & 1), float(vid >> 1));
    float2 px = p.bounds.xy + corner * p.bounds.zw;
    float2 screen = float2(k.layer_matrix.x * px.x + k.layer_matrix.z * px.y, k.layer_matrix.y * px.x + k.layer_matrix.w * px.y)
        + k.layer_translate;
    VsOut o;
    o.pos = float4(screen / k.viewport * float2(2, -2) + float2(-1, 1), 0, 1);
    o.px = px;
    o.uv = mix(p.uv.xy, p.uv.zw, corner);
    o.id = id;
    return o;
}

static float sdf_rrect(float2 p, float2 center, float2 half_size, float4 radii) {
    float2 q = p - center;
    float r = q.x < 0 ? (q.y < 0 ? radii.x : radii.w) : (q.y < 0 ? radii.y : radii.z);
    r = min(r, min(half_size.x, half_size.y));
    float2 d = abs(q) - half_size + r;
    return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0) - r;
}

static float4 premul(float4 c, float coverage) {
    float a = c.a * coverage;
    return float4(c.rgb * a, a);
}

// The gamma and contrast the Windows renderer firms text with: these keep light-on-dark
// text firm without bolding it.
#define TEXT_GAMMA 1.8
#define TEXT_CONTRAST 0.5

// Glyph coverage is a linear area fraction, but it is blended in a gamma-encoded target: as-is
// that thins light text on dark planes and thickens dark text on light ones. The background is
// unknown here, so the correction follows the text's own luminance: light text gets a gamma
// lift, all text a mild contrast boost that firms up stems without growing them.
static float3 text_coverage(float3 a, float3 rgb) {
    float k = TEXT_CONTRAST;
    a = saturate(a * (1 + k) / (1 + k * a));
    float l = dot(rgb, float3(0.2126, 0.7152, 0.0722));
    return mix(a, pow(a, float3(1.0 / TEXT_GAMMA)), l);
}

// Abramowitz-Stegun erf approximation, good to ~1e-4: plenty for shadows.
static float erf_approx(float x) {
    float s = sign(x);
    float a = abs(x);
    float t = 1.0 / (1.0 + 0.3275911 * a);
    float y = 1.0 - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t * exp(-a * a);
    return s * y;
}

// A gradient's colour at a pixel: how far along it the pixel is (params.y: 1 linear at the
// angle params.x, 2 radial, 3 conic from params.x), sampled from its ramp, premultiplied.
static float4 gradient(Instance p, float2 q, float2 half_size, texture2d_array<float> poly, sampler linear) {
    float ang = p.params.x * 0.017453293, along;
    if (p.params.y < 1.5) { // the line's length reaches the corners, as design tools have it
        float2 dir = float2(cos(ang), sin(ang));
        along = dot(q, dir) / max(dot(abs(dir), half_size) * 2, 1.0) + 0.5;
    } else if (p.params.y < 2.5) {
        along = length(q / max(half_size, 0.5)) * 0.70710678;
    } else {
        along = fract((atan2(q.x, -q.y) - ang) * 0.15915494);
    }
    return poly.sample(linear, float2(mix(p.uv.x, p.uv.z, saturate(along)), p.uv.y), p.slice, level(0));
}

// How much of a dashed (flags 4) or dotted (8) border's pixel is ink. The pixel goes by the
// side it is nearest to in that side's widths, seen in a frame where that side is the top and
// runs clockwise. The pattern runs along the middle of the border, round the corners' arcs too,
// stretched to whole periods from the middle of one corner's arc to the middle of the next,
// with a half mark at each end: two sides meet in one mark at a corner.
#define QUARTER 0.78539816
static float border_mark(Instance p, float2 px) {
    float2 q = px - p.bounds.xy, s = p.bounds.zw;
    float4 b = p.border, r = p.radii; // b: t r b l; r: tl tr br bl
    float4 away = float4(q.y, s.x - q.x, s.y - q.y, q.x) / max(b, 1e-3);
    float m = min(min(away.x, away.y), min(away.z, away.w));
    float2 f;
    float len, w, r0, r1;
    if (m == away.x) { f = q; len = s.x; w = b.x; r0 = r.x; r1 = r.y; }
    else if (m == away.y) { f = float2(q.y, s.x - q.x); len = s.y; w = b.y; r0 = r.y; r1 = r.z; }
    else if (m == away.z) { f = s - q; len = s.x; w = b.z; r0 = r.z; r1 = r.w; }
    else { f = float2(s.y - q.y, q.x); len = s.y; w = b.w; r0 = r.w; r1 = r.x; }
    r0 = max(r0, w * 0.5);
    r1 = max(r1, w * 0.5);
    float c0 = QUARTER * (r0 - w * 0.5), c1 = QUARTER * (r1 - w * 0.5);
    float run = c0 + max(len - r0 - r1, 0.0) + c1;
    float along = c0 + f.x - r0, across = f.y;
    if (f.x < r0 && f.y < r0) { // on the arc of the corner the side starts at
        float2 d = r0 - f;
        along = (atan2(d.y, d.x) - QUARTER) * (r0 - w * 0.5);
        across = r0 - length(d);
    } else if (f.x > len - r1 && f.y < r1) { // the one it ends at
        float2 d = float2(f.x - (len - r1), r1 - f.y);
        along = run - (atan2(d.y, d.x) - QUARTER) * (r1 - w * 0.5);
        across = r1 - length(d);
    }
    bool dotted = (p.flags & 8) != 0;
    float period = dotted ? 2 * w : 6 * w;
    period = run / max(round(run / max(period, 1.0)), 1.0);
    float at = fract(along / period) * period;
    float edge = min(at, period - at); // from the nearest end of a period
    if (dotted) return saturate(w * 0.5 + 0.5 - length(float2(edge, across - w * 0.5)));
    return saturate(period * 0.25 + 0.5 - edge);
}

static float4 shade_quad(Instance p, float2 px, float2 screen, constant Constants &k,
                         texture2d_array<float> poly, texture2d<float> backdrop, sampler linear) {
    float2 half_size = p.bounds.zw * 0.5;
    float2 center = p.bounds.xy + half_size;
    float coverage = saturate(0.5 - sdf_rrect(px, center, half_size, p.radii));
    float4 c = premul(p.color, 1);
    if ((p.flags & 1) != 0) c = gradient(p, px - center, half_size, poly, linear);
    if ((p.flags & 16) != 0) { // the fill over a blur of what is behind: the mip of its block
        float lod = clamp(ceil(log2(max(2 * p.params.z, 1.0))), 0.0, 6.0);
        float2 at = (screen - k.backdrop_rect.xy) / max(k.backdrop_rect.zw, 1.0);
        c += backdrop.sample(linear, at, level(lod)) * (1 - c.a);
    }
    if ((p.flags & 2) != 0) {
        float4 b = p.border; // t r b l
        // Sides without a border push the inner edge outside the box, so their
        // antialiasing band never leaks border color along that side.
        float4 none = float4(float(b.x <= 0), float(b.y <= 0), float(b.z <= 0), float(b.w <= 0)) * 2;
        float2 inner_min = p.bounds.xy + float2(b.w, b.x) - float2(none.w, none.x);
        float2 inner_max = p.bounds.xy + p.bounds.zw - float2(b.y, b.z) + float2(none.y, none.z);
        float2 inner_half = max((inner_max - inner_min) * 0.5, 0.0);
        float4 inner_radii = max(p.radii - float4(max(b.w, b.x), max(b.y, b.x), max(b.y, b.z), max(b.w, b.z)), 0.0);
        float inside = saturate(0.5 - sdf_rrect(px, inner_min + inner_half, inner_half, inner_radii));
        float ink = 1;
        if ((p.flags & 12) != 0) ink = border_mark(p, px); // a gap shows the fill
        c = mix(c, premul(p.color2, 1), (1 - inside) * ink);
    }
    return c * coverage;
}

static float4 shade_shadow(Instance p, float2 px) {
    float pad = p.border.x;
    float2 box_min = p.bounds.xy + pad;
    float2 half_size = max(p.bounds.zw * 0.5 - pad, 0.0);
    float d = sdf_rrect(px, box_min + half_size, half_size, p.radii);
    float sigma = max(p.params.x * 0.5, 0.5);
    float a = 0.5 - 0.5 * erf_approx(d / (sigma * 1.41421356));
    return premul(p.color2, a);
}

// column_ink is how much of a chart line's column lies above y: the line, tv tall
// vertically, travels d down the 1 px column, so its coverage is a trapezoid (two boxes
// convolved); this is its integral from the top (r2 is the ramp's antiderivative).
static float r2(float x) {
    x = max(x, 0.0);
    return x * x * 0.5;
}

static float column_ink(float y, float tv, float d) {
    return (r2(y) - r2(y - d) - r2(y - tv) + r2(y - tv - d)) / d;
}

// seg_dist is the distance from p to the segment a-b.
static float seg_dist(float2 p, float2 a, float2 b) {
    float2 pa = p - a, ba = b - a;
    float h = saturate(dot(pa, ba) / max(dot(ba, ba), 1e-6));
    return length(pa - ba * h);
}

static float4 shade_line(Instance p, float2 px) {
    if (p.params.y > 2.5) { // a stroke segment: radii = a, b; uv = prev, next
        float d = seg_dist(px, p.radii.xy, p.radii.zw);
        // The nearest segment draws a joint's pixel (ties go to the later one), so joints
        // are covered once.
        if ((p.flags & 1) != 0 && seg_dist(px, p.uv.xy, p.radii.xy) < d) return float4(0);
        if ((p.flags & 2) != 0 && seg_dist(px, p.radii.zw, p.uv.zw) <= d) return float4(0);
        return premul(p.color, saturate(p.params.x * 0.5 + 0.5 - d));
    }
    if (p.params.y > 1.5) { // a chart line's column (1 px padding above and below)
        float tv = p.params.x, d = max(p.params.z, 0.01);
        float y = px.y - p.bounds.y - 1;
        return premul(p.color, saturate(column_ink(y + 0.5, tv, d) - column_ink(y - 0.5, tv, d)));
    }
    float thickness = max(p.params.x, 1.0);
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

// One output, premultiplied, blended as premultiplied alpha. The Windows shader has a second
// one, a weight a channel, for glyphs smoothed by subpixel: macOS smooths none so, and such a
// glyph, should one come, is blended by the most covered of its channels.
fragment float4 fs_main(VsOut i,
                        const device Instance *instances [[buffer(0)]],
                        constant Constants &k [[buffer(1)]],
                        texture2d_array<float> mono_atlas [[texture(1)]],
                        texture2d_array<float> poly_atlas [[texture(2)]],
                        texture2d<float> backdrop [[texture(3)]],
                        sampler atlas_sampler [[sampler(0)]]) {
    Instance p = instances[i.id];
    float4 c;
    if (p.kind == 0) {
        c = shade_quad(p, i.px, i.pos.xy, k, poly_atlas, backdrop, atlas_sampler);
    } else if (p.kind == 1) {
        c = shade_shadow(p, i.px);
    } else if (p.kind == 2) {
        float cov = mono_atlas.sample(atlas_sampler, i.uv, p.slice).r;
        c = premul(p.color, (p.flags & 1) != 0 ? text_coverage(float3(cov), p.color.rgb).x : cov);
    } else if (p.kind == 3) {
        float2 half_size = p.bounds.zw * 0.5;
        float corner = saturate(0.5 - sdf_rrect(i.px, p.bounds.xy + half_size, half_size, p.radii));
        float4 tile = poly_atlas.sample(atlas_sampler, i.uv, p.slice);
        if ((p.flags & 1) != 0) { // coverage a subpixel (gray when turned)
            float3 cov = k.layer_turned != 0 ? float3(dot(tile.rgb, float3(1.0 / 3))) : tile.rgb;
            float3 w = text_coverage(cov, p.color.rgb) * p.color.a;
            c = float4(p.color.rgb * w, max(w.r, max(w.g, w.b)));
        } else {
            c = tile * p.color.a * corner;
        }
    } else {
        c = shade_line(p, i.px);
    }
    float2 clip_half = p.clip.zw * 0.5;
    float clip = saturate(0.5 - sdf_rrect(i.px, p.clip.xy + clip_half, clip_half, float4(p.params.w))) * k.layer_opacity;
    if (k.mask_on != 0) { // cut to a shape: its coverage here, nothing outside its box
        float2 at = (i.pos.xy - k.mask_rect.xy) / max(k.mask_rect.zw, 1.0);
        float inside = float(all(at >= 0.0) && all(at <= 1.0));
        clip *= inside * mono_atlas.sample(atlas_sampler, mix(k.mask_uv.xy, k.mask_uv.zw, saturate(at)), k.mask_slice, level(0)).r;
    }
    return c * clip;
}
