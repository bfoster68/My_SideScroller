// Parallax rolling hills compute shader
// Terrain detail: water in valleys, snow caps, rock outcrops, wildflowers,
// tree lines, grass fringe, slope shading, atmospheric effects

struct Params {
    camera_x: f32,
    camera_y: f32,
    time: f32,
    resolution_x: f32,
    resolution_y: f32,
    _padding: vec3<f32>,
};

@group(0) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> params: Params;

// ---- Noise functions ----

fn hash(p: vec2<f32>) -> f32 {
    let k = vec2<f32>(0.3183099, 0.3678794);
    var p2 = p * k + k.yx;
    return fract(16.0 * k.x * fract(p2.x * p2.y * (p2.x + p2.y)));
}

fn hash2(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(hash(p), hash(p + vec2(127.1, 311.7)));
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    return mix(
        mix(hash(i + vec2(0.0, 0.0)), hash(i + vec2(1.0, 0.0)), u.x),
        mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x),
        u.y
    );
}

fn fbm(p: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var p2 = p;

    for (var i = 0; i < octaves; i++) {
        value += amplitude * noise(p2 * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

fn rolling_height(x: f32, scale: f32, offset: f32, detail: i32) -> f32 {
    let p = vec2<f32>(x * scale + offset, offset * 0.3);
    let base = noise(p * 0.5) * 0.6;
    let mid = noise(p * 1.2 + vec2(33.0, 0.0)) * 0.25;
    let fine = fbm(p * 2.0 + vec2(77.0, 0.0), detail) * 0.15;
    return base + mid + fine;
}

// Get slope at a point (positive = rising right, negative = falling right)
fn get_slope(x: f32, scale: f32, offset: f32, detail: i32, h_scale: f32) -> f32 {
    let dx = 0.002;
    let h_left = rolling_height(x - dx, scale, offset, detail) * h_scale;
    let h_right = rolling_height(x + dx, scale, offset, detail) * h_scale;
    return (h_right - h_left) / (2.0 * dx);
}

// ---- Day/night cycle ----
// Full cycle = 120 seconds. Phases: night → dawn → day → dusk → night
// 0.0 = midnight, 0.25 = dawn, 0.5 = noon, 0.75 = dusk

fn get_day_phase() -> f32 {
    let cycle_duration = 120.0; // seconds per full cycle
    return fract(params.time / cycle_duration);
}

struct DayColors {
    sky_top: vec3<f32>,
    sky_mid: vec3<f32>,
    sky_horizon: vec3<f32>,
    ambient: vec3<f32>,     // tint applied to all terrain
    light_dir: f32,         // -1 = from left (morning), +1 = from right (evening)
    star_visibility: f32,   // 0 = no stars, 1 = full stars
    moon_visibility: f32,
    sun_visibility: f32,
    sun_pos: vec2<f32>,
};

fn get_day_colors() -> DayColors {
    let phase = get_day_phase();
    var dc: DayColors;

    // Night: 0.0 - 0.15
    let night_sky_top = vec3<f32>(0.01, 0.01, 0.06);
    let night_sky_mid = vec3<f32>(0.04, 0.03, 0.12);
    let night_sky_horizon = vec3<f32>(0.08, 0.06, 0.16);
    let night_ambient = vec3<f32>(0.7, 0.75, 1.0);

    // Dawn: 0.2 - 0.3
    let dawn_sky_top = vec3<f32>(0.15, 0.10, 0.25);
    let dawn_sky_mid = vec3<f32>(0.35, 0.18, 0.22);
    let dawn_sky_horizon = vec3<f32>(0.55, 0.30, 0.15);
    let dawn_ambient = vec3<f32>(1.0, 0.85, 0.75);

    // Day: 0.4 - 0.6
    let day_sky_top = vec3<f32>(0.15, 0.25, 0.55);
    let day_sky_mid = vec3<f32>(0.30, 0.45, 0.65);
    let day_sky_horizon = vec3<f32>(0.50, 0.60, 0.70);
    let day_ambient = vec3<f32>(1.0, 1.0, 0.95);

    // Dusk: 0.7 - 0.8
    let dusk_sky_top = vec3<f32>(0.10, 0.06, 0.20);
    let dusk_sky_mid = vec3<f32>(0.30, 0.12, 0.18);
    let dusk_sky_horizon = vec3<f32>(0.50, 0.20, 0.10);
    let dusk_ambient = vec3<f32>(1.0, 0.80, 0.70);

    if phase < 0.15 {
        // Deep night
        dc.sky_top = night_sky_top;
        dc.sky_mid = night_sky_mid;
        dc.sky_horizon = night_sky_horizon;
        dc.ambient = night_ambient;
        dc.light_dir = -0.3;
        dc.star_visibility = 1.0;
        dc.moon_visibility = 1.0;
        dc.sun_visibility = 0.0;
        dc.sun_pos = vec2<f32>(0.0, 0.0);
    } else if phase < 0.30 {
        // Dawn transition
        let t = (phase - 0.15) / 0.15;
        let t_smooth = t * t * (3.0 - 2.0 * t);
        dc.sky_top = mix(night_sky_top, dawn_sky_top, t_smooth);
        dc.sky_mid = mix(night_sky_mid, dawn_sky_mid, t_smooth);
        dc.sky_horizon = mix(night_sky_horizon, dawn_sky_horizon, t_smooth);
        dc.ambient = mix(night_ambient, dawn_ambient, t_smooth);
        dc.light_dir = mix(-0.3, -0.8, t_smooth);
        dc.star_visibility = 1.0 - t_smooth;
        dc.moon_visibility = 1.0 - t_smooth;
        dc.sun_visibility = t_smooth * 0.5;
        dc.sun_pos = vec2<f32>(0.1 + t_smooth * 0.15, 0.3 + t_smooth * 0.3);
    } else if phase < 0.45 {
        // Dawn to day
        let t = (phase - 0.30) / 0.15;
        let t_smooth = t * t * (3.0 - 2.0 * t);
        dc.sky_top = mix(dawn_sky_top, day_sky_top, t_smooth);
        dc.sky_mid = mix(dawn_sky_mid, day_sky_mid, t_smooth);
        dc.sky_horizon = mix(dawn_sky_horizon, day_sky_horizon, t_smooth);
        dc.ambient = mix(dawn_ambient, day_ambient, t_smooth);
        dc.light_dir = mix(-0.8, -0.5, t_smooth);
        dc.star_visibility = 0.0;
        dc.moon_visibility = 0.0;
        dc.sun_visibility = 0.5 + t_smooth * 0.5;
        dc.sun_pos = vec2<f32>(0.25 + t_smooth * 0.25, 0.6 + t_smooth * 0.2);
    } else if phase < 0.60 {
        // Full day
        dc.sky_top = day_sky_top;
        dc.sky_mid = day_sky_mid;
        dc.sky_horizon = day_sky_horizon;
        dc.ambient = day_ambient;
        dc.light_dir = 0.0;
        dc.star_visibility = 0.0;
        dc.moon_visibility = 0.0;
        dc.sun_visibility = 1.0;
        let t = (phase - 0.45) / 0.15;
        dc.sun_pos = vec2<f32>(0.5, 0.82 - abs(t - 0.5) * 0.1);
    } else if phase < 0.75 {
        // Day to dusk
        let t = (phase - 0.60) / 0.15;
        let t_smooth = t * t * (3.0 - 2.0 * t);
        dc.sky_top = mix(day_sky_top, dusk_sky_top, t_smooth);
        dc.sky_mid = mix(day_sky_mid, dusk_sky_mid, t_smooth);
        dc.sky_horizon = mix(day_sky_horizon, dusk_sky_horizon, t_smooth);
        dc.ambient = mix(day_ambient, dusk_ambient, t_smooth);
        dc.light_dir = mix(0.0, 0.8, t_smooth);
        dc.star_visibility = 0.0;
        dc.moon_visibility = 0.0;
        dc.sun_visibility = 1.0 - t_smooth * 0.5;
        dc.sun_pos = vec2<f32>(0.75 + t_smooth * 0.15, 0.6 - t_smooth * 0.3);
    } else if phase < 0.90 {
        // Dusk to night
        let t = (phase - 0.75) / 0.15;
        let t_smooth = t * t * (3.0 - 2.0 * t);
        dc.sky_top = mix(dusk_sky_top, night_sky_top, t_smooth);
        dc.sky_mid = mix(dusk_sky_mid, night_sky_mid, t_smooth);
        dc.sky_horizon = mix(dusk_sky_horizon, night_sky_horizon, t_smooth);
        dc.ambient = mix(dusk_ambient, night_ambient, t_smooth);
        dc.light_dir = mix(0.8, 0.3, t_smooth);
        dc.star_visibility = t_smooth;
        dc.moon_visibility = t_smooth;
        dc.sun_visibility = 0.5 - t_smooth * 0.5;
        dc.sun_pos = vec2<f32>(0.9, 0.3 - t_smooth * 0.2);
    } else {
        // Deep night again
        dc.sky_top = night_sky_top;
        dc.sky_mid = night_sky_mid;
        dc.sky_horizon = night_sky_horizon;
        dc.ambient = night_ambient;
        dc.light_dir = -0.3;
        dc.star_visibility = 1.0;
        dc.moon_visibility = 1.0;
        dc.sun_visibility = 0.0;
        dc.sun_pos = vec2<f32>(0.0, 0.0);
    }

    return dc;
}

// Apply ambient tint to terrain colors
fn tint_color(col: vec3<f32>, ambient: vec3<f32>) -> vec3<f32> {
    return col * ambient;
}

// ---- Tree silhouettes (with wind sway) ----

fn tree_line(x: f32, hill_y: f32, uv_y: f32, scale: f32, seed: f32) -> f32 {
    let tree_x = x * scale * 40.0 + seed;
    let grid_x = floor(tree_x);
    let frac_x = fract(tree_x);
    let h = hash(vec2(grid_x, seed));

    if h < 0.4 { return 0.0; }

    let tree_center = 0.5 + (h - 0.5) * 0.3;
    let dist_x = abs(frac_x - tree_center);
    let tree_height = (0.008 + h * 0.012) * (0.8 + hash(vec2(grid_x + 100.0, seed)) * 0.4);
    let tree_width = 0.25 + h * 0.15;

    let tree_top = hill_y + tree_height;

    // Wind sway: offset increases with height in tree
    let wind_speed = 1.5 + hash(vec2(grid_x, seed + 50.0)) * 1.0;
    let wind_phase = hash(vec2(grid_x, seed + 100.0)) * 6.28;

    if uv_y < tree_top && uv_y > hill_y && dist_x < tree_width {
        let local_y = (uv_y - hill_y) / tree_height;
        // Sway increases toward treetop
        let sway = sin(params.time * wind_speed + wind_phase) * local_y * local_y * 0.08;
        let swayed_dist_x = abs(frac_x - tree_center - sway);

        if swayed_dist_x < tree_width * (1.0 - local_y * 0.8) {
            return 1.0;
        }
    }
    return 0.0;
}

// ---- Fireflies (near layer) ----

fn fireflies(uv: vec2<f32>, camera_x: f32) -> vec3<f32> {
    var result = vec3<f32>(0.0);

    for (var i = 0; i < 20; i++) {
        let seed = f32(i) * 47.3;

        // Base position with parallax
        let base_x = hash(vec2(seed, 1.0)) * 3.0 - 0.5; // spread over wider area
        let base_y = 0.04 + hash(vec2(seed, 2.0)) * 0.20; // low near ground

        // Gentle drifting motion (figure-8 / lissajous)
        let freq_x = 0.3 + hash(vec2(seed, 3.0)) * 0.4;
        let freq_y = 0.4 + hash(vec2(seed, 4.0)) * 0.3;
        let drift_x = sin(params.time * freq_x + seed) * 0.02;
        let drift_y = sin(params.time * freq_y + seed * 1.3) * 0.015;

        let fly_pos = vec2<f32>(
            base_x + drift_x + camera_x * 0.20 / params.resolution_x,
            base_y + drift_y,
        );

        let dist = length(uv - fly_pos);

        // Pulsing glow
        let pulse_speed = 1.5 + hash(vec2(seed, 5.0)) * 2.0;
        let pulse_phase = hash(vec2(seed, 6.0)) * 6.28;
        let pulse = 0.3 + 0.7 * max(0.0, sin(params.time * pulse_speed + pulse_phase));

        // Soft glow radius
        let glow_size = 0.008;
        if dist < glow_size * 3.0 {
            let brightness = smoothstep(glow_size * 3.0, 0.0, dist) * pulse;
            // Warm yellow-green glow
            result += vec3<f32>(0.15, 0.25, 0.05) * brightness;
        }
    }

    return result;
}

// ---- Grass fringe ----

fn grass_fringe(x: f32, hill_y: f32, uv_y: f32, scale: f32, seed: f32) -> f32 {
    let gx = x * scale * 80.0 + seed;
    let h = hash(vec2(floor(gx), seed * 1.7));
    let blade_h = 0.003 + h * 0.005;
    let top = hill_y + blade_h;

    if uv_y > hill_y && uv_y < top {
        let frac = fract(gx);
        let center_dist = abs(frac - 0.5);
        if center_dist < 0.15 {
            return 1.0;
        }
    }
    return 0.0;
}

// ---- Snow caps on peaks ----

fn snow_cap(x: f32, hill_y: f32, uv_y: f32, h_scale: f32, scale: f32, offset: f32, peak_threshold: f32) -> vec3<f32> {
    // Only apply snow near local peaks (where slope is near zero and height is above threshold)
    if hill_y < peak_threshold { return vec3<f32>(-1.0); }

    let slope = abs(get_slope(x, scale, offset, 2, h_scale));
    // Flatter = more snow, steep = no snow
    let snow_coverage = smoothstep(0.8, 0.1, slope);
    if snow_coverage < 0.05 { return vec3<f32>(-1.0); }

    // Snow only on top portion of peak
    let snow_depth = 0.015 * snow_coverage;
    let dist_from_top = hill_y - uv_y;

    if dist_from_top > 0.0 && dist_from_top < snow_depth {
        // Noisy snow edge
        let edge_noise = hash(vec2(x * 200.0, offset)) * 0.005;
        if dist_from_top < snow_depth - edge_noise {
            let brightness = 0.6 + snow_coverage * 0.3;
            // Slightly blue-tinted snow
            return vec3<f32>(brightness * 0.9, brightness * 0.92, brightness);
        }
    }
    return vec3<f32>(-1.0);
}

// ---- Rock outcrops on steep slopes ----

fn rock_outcrop(x: f32, uv_y: f32, hill_y: f32, h_scale: f32, scale: f32, offset: f32, detail: i32) -> vec3<f32> {
    let slope = abs(get_slope(x, scale, offset, detail, h_scale));

    // Only on steep slopes
    if slope < 1.2 { return vec3<f32>(-1.0); }

    let depth = hill_y - uv_y;
    if depth < 0.0 || depth > h_scale * 0.3 { return vec3<f32>(-1.0); }

    // Rock pattern: irregular patches using high-freq noise
    let rock_noise = fbm(vec2(x * scale * 15.0 + offset, uv_y * 30.0), 3);
    let rock_threshold = smoothstep(1.2, 2.5, slope) * 0.6;

    if rock_noise > (1.0 - rock_threshold) {
        // Rocky grey with slight variation
        let grey = 0.08 + rock_noise * 0.04;
        let variation = hash(vec2(x * 100.0, uv_y * 100.0)) * 0.02;
        return vec3<f32>(grey + variation, grey, grey - variation * 0.5);
    }
    return vec3<f32>(-1.0);
}

// ---- Wildflowers scattered on near ridges ----

fn wildflowers(x: f32, uv_y: f32, hill_y: f32, scale: f32, seed: f32) -> vec3<f32> {
    let flower_grid = vec2<f32>(x * scale * 120.0 + seed, uv_y * 200.0);
    let cell = floor(flower_grid);
    let h = hash(cell);

    // Only ~8% of cells get a flower, only near the ridge
    let dist_from_ridge = hill_y - uv_y;
    if h > 0.92 && dist_from_ridge > 0.0 && dist_from_ridge < 0.02 {
        let frac = fract(flower_grid);
        let center_dist = length(frac - vec2(0.5));

        if center_dist < 0.2 {
            // Different flower colors based on hash
            let color_pick = hash(cell + vec2(99.0, 77.0));
            if color_pick < 0.3 {
                // Purple
                return vec3<f32>(0.25, 0.08, 0.30);
            } else if color_pick < 0.55 {
                // Yellow
                return vec3<f32>(0.35, 0.30, 0.05);
            } else if color_pick < 0.75 {
                // Red/pink
                return vec3<f32>(0.30, 0.06, 0.08);
            } else {
                // White
                return vec3<f32>(0.35, 0.35, 0.32);
            }
        }
    }
    return vec3<f32>(-1.0);
}

// ---- Water in valleys ----

fn valley_water(
    uv: vec2<f32>,
    world_x: f32,
    scale: f32,
    offset: f32,
    detail: i32,
    h_scale: f32,
    h_base: f32,
    water_level: f32,
) -> vec3<f32> {
    let h = rolling_height(world_x, scale, offset, detail) * h_scale + h_base;

    // Water fills valleys below a threshold
    if uv.y >= water_level || uv.y >= h {
        return vec3<f32>(-1.0);
    }

    // Water surface properties
    let water_depth = water_level - uv.y;
    let surface_dist = water_level - uv.y;

    // Ripple animation
    let ripple_x = world_x * scale * 30.0 + params.time * 0.5;
    let ripple = sin(ripple_x) * 0.3 + sin(ripple_x * 2.3 + 1.0) * 0.15;

    // Base water color (dark, reflective)
    let water_deep = vec3<f32>(0.02, 0.04, 0.08);
    let water_surface = vec3<f32>(0.05, 0.08, 0.15);

    var water_col = mix(water_surface, water_deep, smoothstep(0.0, 0.03, water_depth));

    // Surface highlights (moonlight reflections)
    let highlight = smoothstep(0.003, 0.0, surface_dist) * (0.3 + ripple * 0.15);
    water_col += vec3<f32>(0.08, 0.07, 0.12) * highlight;

    // Subtle shimmer
    let shimmer = hash(vec2(world_x * 50.0 + params.time * 0.3, uv.y * 100.0));
    if shimmer > 0.97 && surface_dist < 0.005 {
        water_col += vec3<f32>(0.1, 0.09, 0.14) * (shimmer - 0.97) * 30.0;
    }

    return water_col;
}

// ---- Star field ----

fn star_field(uv: vec2<f32>, camera_x: f32) -> f32 {
    let parallax_x = camera_x * 0.01 / params.resolution_x;
    let grid = floor((uv + vec2(parallax_x, 0.0)) * 80.0);
    let h = hash(grid);

    if h > 0.97 {
        let star_pos = fract((uv + vec2(parallax_x, 0.0)) * 80.0);
        let dist = length(star_pos - vec2(0.5, 0.5));
        let brightness = smoothstep(0.15, 0.0, dist);
        let twinkle = 0.7 + 0.3 * sin(params.time * (2.0 + h * 5.0) + h * 100.0);
        return brightness * twinkle * (0.5 + h * 0.5);
    }
    return 0.0;
}

// ---- Shooting stars ----

fn shooting_star(uv: vec2<f32>, camera_x: f32) -> vec3<f32> {
    // Cycle through shooting star "slots" — each slot has a unique timing
    var result = vec3<f32>(0.0);

    for (var i = 0; i < 3; i++) {
        let seed = f32(i) * 173.7;
        // Each shooting star has a random period (8-15 seconds)
        let period = 8.0 + hash(vec2(seed, 0.0)) * 7.0;
        // Active for only ~0.6 seconds of each period
        let phase = (params.time + seed * 3.0) % period;
        let active_duration = 0.6;

        if phase > active_duration { continue; }

        let t = phase / active_duration; // 0..1 progress

        // Random start position (upper sky region)
        let start_x = hash(vec2(seed + 1.0, floor(params.time / period))) * 0.8 + 0.1;
        let start_y = 0.7 + hash(vec2(seed + 2.0, floor(params.time / period))) * 0.25;

        // Direction: downward-right at ~30-45 degrees
        let angle = 0.4 + hash(vec2(seed + 3.0, floor(params.time / period))) * 0.3;
        let speed = 0.4 + hash(vec2(seed + 4.0, floor(params.time / period))) * 0.3;
        let dir = vec2<f32>(cos(angle), -sin(angle)) * speed;

        // Current head position
        let head = vec2<f32>(start_x, start_y) + dir * t;

        // Parallax offset for stars layer
        let parallax_offset = vec2<f32>(camera_x * 0.01 / params.resolution_x, 0.0);
        let adjusted_uv = uv + parallax_offset;

        // Trail: check distance from the line segment (head to tail)
        let trail_length = 0.08 + t * 0.04; // trail grows slightly
        let tail = head - normalize(dir) * trail_length;

        // Point-to-line-segment distance
        let pa = adjusted_uv - tail;
        let ba = head - tail;
        let h_param = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
        let dist = length(pa - ba * h_param);

        // Thickness tapers from head to tail
        let thickness = 0.003 * (1.0 - h_param * 0.7);

        if dist < thickness {
            let brightness = smoothstep(thickness, 0.0, dist);
            // Fade along trail (bright at head, dim at tail)
            let trail_fade = h_param * h_param;
            // Fade in/out over lifetime
            let life_fade = sin(t * 3.14159);
            let intensity = brightness * trail_fade * life_fade * 0.8;
            result += vec3<f32>(0.7, 0.75, 1.0) * intensity;
        }
    }
    return result;
}

// ---- Drifting clouds ----

fn cloud_layer(uv: vec2<f32>, camera_x: f32, speed: f32, parallax: f32, y_center: f32, thickness: f32, density: f32, scale: f32) -> f32 {
    // Cloud position with parallax and drift
    let drift = params.time * speed;
    let cx = uv.x + camera_x * parallax / params.resolution_x + drift;
    let cy = uv.y;

    // Cloud shape from layered noise
    let p = vec2<f32>(cx * scale, cy * scale * 3.0); // stretch horizontally
    let n = fbm(p, 5);

    // Vertical falloff (clouds are thin bands)
    let y_dist = abs(cy - y_center) / thickness;
    let y_falloff = smoothstep(1.0, 0.3, y_dist);

    // Cloud threshold — only render dense enough areas
    let cloud_val = (n - (1.0 - density)) * y_falloff;

    return max(cloud_val, 0.0);
}

fn render_clouds(uv: vec2<f32>, camera_x: f32, current_color: vec3<f32>) -> vec3<f32> {
    var color = current_color;

    // High wispy cirrus clouds (very slow, subtle)
    let cirrus = cloud_layer(uv, camera_x,
        0.003,   // drift speed
        0.01,    // parallax
        0.78,    // y center
        0.12,    // thickness
        0.35,    // density
        2.0,     // scale
    );
    if cirrus > 0.0 {
        let cirrus_color = vec3<f32>(0.08, 0.07, 0.14);
        color = mix(color, cirrus_color, cirrus * 0.3);
    }

    // Mid-level clouds (moderate drift, more opaque)
    let mid_cloud = cloud_layer(uv, camera_x,
        0.008,   // drift speed
        0.04,    // parallax
        0.58,    // y center
        0.08,    // thickness
        0.40,    // density
        3.0,     // scale
    );
    if mid_cloud > 0.0 {
        let mid_color = vec3<f32>(0.06, 0.05, 0.10);
        // Slight bright edge on top (moonlit)
        let edge_bright = smoothstep(0.0, 0.02, mid_cloud) * 0.15;
        let lit_color = mid_color + vec3<f32>(edge_bright, edge_bright * 0.8, edge_bright * 1.2);
        color = mix(color, lit_color, mid_cloud * 0.4);
    }

    // Low scattered clouds (faster drift, between hill layers)
    let low_cloud = cloud_layer(uv, camera_x,
        0.015,   // drift speed
        0.08,    // parallax
        0.38,    // y center
        0.06,    // thickness
        0.45,    // density
        4.0,     // scale
    );
    if low_cloud > 0.0 {
        let low_color = vec3<f32>(0.05, 0.04, 0.08);
        color = mix(color, low_color, low_cloud * 0.35);
    }

    return color;
}

// ---- Fog wisps rising from valleys ----

fn valley_fog(uv: vec2<f32>, world_x: f32, scale: f32, offset: f32, detail: i32, h_scale: f32, h_base: f32) -> f32 {
    let h = rolling_height(world_x, scale, offset, detail) * h_scale + h_base;

    // Fog only in valleys (where terrain is low)
    let valley_depth = 0.3 - h; // deeper valleys = more fog
    if valley_depth < 0.0 { return 0.0; }

    // Fog rises above the terrain
    let fog_height_above = uv.y - h;
    if fog_height_above < 0.0 || fog_height_above > 0.08 { return 0.0; }

    // Animated rising wisps
    let wisp_x = world_x * scale * 8.0 + offset;
    let wisp_y = uv.y * 20.0 - params.time * 0.3; // drift upward
    let wisp_noise = fbm(vec2(wisp_x, wisp_y), 3);

    // Vertical falloff (densest at terrain, fading upward)
    let vert_fade = smoothstep(0.08, 0.0, fog_height_above);

    // Valley intensity
    let valley_factor = smoothstep(0.0, 0.15, valley_depth);

    return wisp_noise * vert_fade * valley_factor * 0.5;
}

// ---- God rays from moon ----

fn god_rays(uv: vec2<f32>) -> f32 {
    let moon_pos = vec2<f32>(0.75, 0.82);
    let dir = uv - moon_pos;
    let angle = atan2(dir.y, dir.x);
    let dist = length(dir);

    // Radial rays using noise on the angle
    let ray_noise = noise(vec2(angle * 8.0 + 0.5, params.time * 0.05));
    let ray_pattern = smoothstep(0.3, 0.7, ray_noise);

    // Fade with distance from moon
    let dist_fade = smoothstep(0.6, 0.0, dist);

    // Only cast downward (below the moon)
    let downward = smoothstep(0.82, 0.5, uv.y);

    return ray_pattern * dist_fade * downward * 0.08;
}

// ---- Full hill layer renderer ----

fn render_hill_layer(
    uv: vec2<f32>,
    world_x: f32,
    scale: f32,
    offset: f32,
    detail: i32,
    height_scale: f32,
    height_base: f32,
    color_top: vec3<f32>,
    color_bottom: vec3<f32>,
    color_shadow: vec3<f32>,
    has_trees: bool,
    has_grass: bool,
    has_snow: bool,
    snow_threshold: f32,
    has_rocks: bool,
    has_flowers: bool,
    water_level: f32,  // set to 0.0 for no water
    current_color: vec3<f32>,
) -> vec3<f32> {
    let h = rolling_height(world_x, scale, offset, detail) * height_scale + height_base;

    // --- Water in valleys (render behind / below hills) ---
    if water_level > 0.0 {
        let water = valley_water(uv, world_x, scale, offset, detail, height_scale, height_base, water_level);
        if water.x >= 0.0 {
            return water;
        }
    }

    if uv.y > h + 0.025 {
        return current_color;
    }

    var col = current_color;

    // Trees on the ridgeline
    if has_trees {
        let t = tree_line(world_x, h, uv.y, scale, offset);
        if t > 0.0 {
            // Vary tree darkness slightly
            let tree_var = hash(vec2(world_x * 100.0, offset)) * 0.02;
            col = color_shadow + vec3<f32>(0.0, tree_var, 0.0);
            return col;
        }
    }

    // Grass fringe
    if has_grass {
        let g = grass_fringe(world_x, h, uv.y, scale, offset);
        if g > 0.0 {
            col = mix(color_top, color_shadow, 0.5);
            return col;
        }
    }

    if uv.y >= h {
        return current_color;
    }

    // --- Below the ridge ---
    let depth = (h - uv.y) / height_scale;

    // Base gradient
    col = mix(color_top, color_bottom, smoothstep(0.0, 0.5, depth));

    // Slope shading (moonlight from upper-left)
    let slope = get_slope(world_x, scale, offset, detail, height_scale);
    let shade = clamp(0.5 - slope * 0.3, 0.0, 1.0);
    col = mix(color_shadow, col, shade);

    // Rim light at ridge
    let rim = smoothstep(0.02, 0.0, h - uv.y);
    col = mix(col, color_top * 1.2, rim * 0.3);

    // --- Snow caps ---
    if has_snow {
        let snow = snow_cap(world_x, h, uv.y, height_scale, scale, offset, snow_threshold);
        if snow.x >= 0.0 {
            col = snow;
        }
    }

    // --- Rock outcrops ---
    if has_rocks {
        let rock = rock_outcrop(world_x, uv.y, h, height_scale, scale, offset, detail);
        if rock.x >= 0.0 {
            col = rock;
        }
    }

    // --- Wildflowers ---
    if has_flowers {
        let flower = wildflowers(world_x, uv.y, h, scale, offset);
        if flower.x >= 0.0 {
            col = flower;
        }
    }

    return col;
}

// =============== MAIN ===============

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = vec2<f32>(params.resolution_x, params.resolution_y);
    let pixel = vec2<f32>(f32(global_id.x), f32(global_id.y));

    if pixel.x >= dims.x || pixel.y >= dims.y {
        return;
    }

    let uv = vec2<f32>(pixel.x / dims.x, 1.0 - pixel.y / dims.y);

    // === Day/night cycle ===
    let dc = get_day_colors();

    // === Sky gradient (driven by cycle) ===
    var color: vec3<f32>;
    if uv.y > 0.5 {
        color = mix(dc.sky_mid, dc.sky_top, (uv.y - 0.5) * 2.0);
    } else {
        color = mix(dc.sky_horizon, dc.sky_mid, uv.y * 2.0);
    }

    // === Sun ===
    if dc.sun_visibility > 0.0 {
        let sun_dist = length(uv - dc.sun_pos);
        // Sun glow
        let sun_glow = smoothstep(0.2, 0.0, sun_dist) * 0.3 * dc.sun_visibility;
        color += vec3<f32>(0.5, 0.4, 0.2) * sun_glow;
        // Sun disk
        if sun_dist < 0.04 {
            let sun_bright = smoothstep(0.04, 0.02, sun_dist) * dc.sun_visibility;
            color += vec3<f32>(1.0, 0.9, 0.6) * sun_bright;
        }
    }

    // === Stars (fade with day) ===
    if dc.star_visibility > 0.0 && uv.y > 0.35 {
        let star = star_field(uv, params.camera_x);
        color += vec3<f32>(0.8, 0.85, 1.0) * star * smoothstep(0.35, 0.65, uv.y) * dc.star_visibility;
    }

    // === Shooting stars (night only) ===
    if dc.star_visibility > 0.3 {
        color += shooting_star(uv, params.camera_x) * dc.star_visibility;
    }

    // === Moon (fade with day) ===
    if dc.moon_visibility > 0.0 {
        let moon_pos = vec2<f32>(0.75, 0.82);
        let moon_dist = length(uv - moon_pos);
        color += vec3<f32>(0.15, 0.12, 0.25) * smoothstep(0.15, 0.0, moon_dist) * 0.15 * dc.moon_visibility;
        if moon_dist < 0.03 {
            color += vec3<f32>(0.25, 0.22, 0.35) * smoothstep(0.03, 0.015, moon_dist) * dc.moon_visibility;
        }

        // === God rays from moon (night only) ===
        let rays = god_rays(uv);
        color += vec3<f32>(0.12, 0.10, 0.18) * rays * dc.moon_visibility;
    }

    // === High clouds (behind far hills) ===
    color = render_clouds(uv, params.camera_x, color);

    // === Layer 1: Far hills — misty, snow-capped peaks, no vegetation ===
    let far_x = pixel.x / dims.x + params.camera_x * 0.03 / dims.x;
    color = render_hill_layer(
        uv, far_x,
        1.5, 42.0, 2,
        0.35, 0.40,
        tint_color(vec3<f32>(0.10, 0.08, 0.18), dc.ambient),
        tint_color(vec3<f32>(0.06, 0.05, 0.12), dc.ambient),
        tint_color(vec3<f32>(0.04, 0.04, 0.10), dc.ambient),
        false, false,
        true, 0.48,
        false, false,
        0.0,
        color,
    );

    // === Fog wisps in far valleys ===
    let fog_far = valley_fog(uv, far_x, 1.5, 42.0, 2, 0.35, 0.40);
    if fog_far > 0.0 {
        color = mix(color, tint_color(vec3<f32>(0.10, 0.08, 0.16), dc.ambient), fog_far);
    }

    // === Layer 2: Mid-far hills — trees, rock outcrops ===
    let midfar_x = pixel.x / dims.x + params.camera_x * 0.07 / dims.x;
    color = render_hill_layer(
        uv, midfar_x,
        2.0, 137.0, 3,
        0.28, 0.33,
        tint_color(vec3<f32>(0.08, 0.10, 0.14), dc.ambient),
        tint_color(vec3<f32>(0.05, 0.06, 0.10), dc.ambient),
        tint_color(vec3<f32>(0.03, 0.04, 0.08), dc.ambient),
        true, false,
        true, 0.38,
        true, false,
        0.20,
        color,
    );

    // === Fog wisps in mid-far valleys ===
    let fog_midfar = valley_fog(uv, midfar_x, 2.0, 137.0, 3, 0.28, 0.33);
    if fog_midfar > 0.0 {
        color = mix(color, tint_color(vec3<f32>(0.08, 0.07, 0.13), dc.ambient), fog_midfar);
    }

    // === Layer 3: Mid hills — full vegetation, flowers, water ===
    let mid_x = pixel.x / dims.x + params.camera_x * 0.12 / dims.x;
    color = render_hill_layer(
        uv, mid_x,
        2.5, 89.0, 4,
        0.22, 0.27,
        tint_color(vec3<f32>(0.06, 0.12, 0.08), dc.ambient),
        tint_color(vec3<f32>(0.03, 0.07, 0.05), dc.ambient),
        tint_color(vec3<f32>(0.02, 0.04, 0.03), dc.ambient),
        true, true,
        false, 0.0,
        true, true,
        0.14,
        color,
    );

    // === Fog wisps in mid valleys ===
    let fog_mid = valley_fog(uv, mid_x, 2.5, 89.0, 4, 0.22, 0.27);
    if fog_mid > 0.0 {
        color = mix(color, tint_color(vec3<f32>(0.06, 0.06, 0.10), dc.ambient), fog_mid);
    }

    // === Layer 4: Near hills — dense detail, silhouette weight ===
    let near_x = pixel.x / dims.x + params.camera_x * 0.20 / dims.x;
    color = render_hill_layer(
        uv, near_x,
        3.0, 211.0, 5,
        0.18, 0.20,
        tint_color(vec3<f32>(0.04, 0.08, 0.05), dc.ambient),
        tint_color(vec3<f32>(0.02, 0.04, 0.03), dc.ambient),
        tint_color(vec3<f32>(0.01, 0.02, 0.02), dc.ambient),
        true, true,
        false, 0.0,
        true, true,
        0.0,
        color,
    );

    // === Fireflies (near ground, night/dusk only) ===
    if dc.star_visibility > 0.1 {
        color += fireflies(uv, params.camera_x) * dc.star_visibility;
    }

    // === Ground fill ===
    if uv.y < 0.20 {
        color = tint_color(vec3<f32>(0.01, 0.02, 0.02), dc.ambient);
    }

    // === Atmospheric haze (tinted by time of day) ===
    let haze_factor = smoothstep(0.25, 0.0, uv.y) * 0.25;
    color = mix(color, tint_color(vec3<f32>(0.08, 0.06, 0.14), dc.ambient), haze_factor);

    // === Vignette ===
    let vignette_dist = length((uv - vec2(0.5, 0.5)) * vec2(1.0, 0.7));
    color *= 1.0 - smoothstep(0.5, 1.0, vignette_dist) * 0.3;

    textureStore(output_texture, vec2<i32>(global_id.xy), vec4<f32>(color, 1.0));
}
