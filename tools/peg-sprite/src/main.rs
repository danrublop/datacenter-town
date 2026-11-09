//! Draws one pixel-art peg person (round head, body, arms, legs) and
//! palette-swaps it into every agent color.
//!
//! Output, written next to the map:
//!   pegs.png         sheet: one block per color, 4 rows (down, left, right, up)
//!                    x 3 walk frames (stand, step A, step B), 16x24 px each
//!   pegs-preview.png the same sheet scaled 6x, for looking at
//!
//!   cargo run --release -- <out_dir>

use std::fs::File;
use std::io::BufWriter;

const FW: usize = 16;
const FH: usize = 24;
const FRAMES: usize = 3;
const DIRS: usize = 4; // down, left, right, up

// Same order as the dashboard charts; the last one is red here, olive in the charts.
const COLORS: [u32; 8] = [
    0x2f7d52, 0xb4552d, 0x3b6ea5, 0x8a5bb0, 0xb08b1e, 0x2a8f8f, 0xa8456b, 0xc8333a,
];

// Eye style baked into pegs.png. 0 none, 1 dots, 2 tall, 3 dots lower,
// 4 wide-set, 5 shiny, 6 dots + smile, 7 happy ^ ^.
const EYES: u8 = 2;

// Tones painted on the template; each becomes a shade of the agent color.
#[derive(Clone, Copy, PartialEq)]
enum T { Empty, Outline, Shadow, Mid, Light, Shine, Eye, Glint }

type Frame = [[T; FW]; FH];

#[derive(Clone, Copy, PartialEq)]
enum Dir { Down, Left, Right, Up }

fn fill(f: &mut Frame, x0: i32, y0: i32, x1: i32, y1: i32) {
    for y in y0.max(0)..=y1.min(FH as i32 - 1) {
        for x in x0.max(0)..=x1.min(FW as i32 - 1) {
            f[y as usize][x as usize] = T::Light;
        }
    }
}

fn frame(dir: Dir, step: usize, eyes: u8) -> Frame {
    let mut f = [[T::Empty; FW]; FH];
    // Walk: step 1 = one foot forward, step 2 = the other.
    let swing: i32 = match step { 1 => 1, 2 => -1, _ => 0 };
    let bob: i32 = if step == 0 { 0 } else { -1 };

    let top = 11 + bob; // body starts under the head
    let mut arms: Vec<(i32, i32, i32, i32)> = vec![];
    match dir {
        Dir::Down | Dir::Up => {
            fill(&mut f, 5, top, 10, top + 6); // torso
            // arms hang beside the torso, swinging a pixel when walking
            fill(&mut f, 2, top + 1 - swing.min(0), 3, top + 6 - swing.min(0));
            fill(&mut f, 12, top + 1 + swing.max(0), 13, top + 6 + swing.max(0));
            // legs with a gap; the stepping leg lifts a pixel
            fill(&mut f, 5, top + 7, 6, 21 - (swing < 0) as i32);
            fill(&mut f, 9, top + 7, 10, 21 - (swing > 0) as i32);
        }
        Dir::Left | Dir::Right => {
            fill(&mut f, 5, top, 10, top + 6);
            // legs spread front/back while walking
            if swing == 0 {
                fill(&mut f, 7, top + 7, 9, 21); // standing: legs together
            } else {
                // striding: legs angle apart, one step back and one forward
                fill(&mut f, 6, top + 7, 7, top + 8); fill(&mut f, 5, top + 9, 6, 21);
                fill(&mut f, 9, top + 7, 10, top + 8); fill(&mut f, 10, top + 9, 11, 21);
            }
            // one arm in front of the body, swinging opposite the legs
            arms.push((7 + 2 * swing, top + 1, 8 + 2 * swing, top + 6));
        }
    }
    // Mirror the shape for Left before shading, so the light stays upper right.
    if dir == Dir::Left {
        for row in f.iter_mut() { row.reverse(); }
        let w = FW as i32 - 1;
        for a in arms.iter_mut() { *a = (w - a.2, a.1, w - a.0, a.3); }
    }
    shade(&mut f);
    // Side arm drawn over the torso with its own outline.
    for &(x0, y0, x1, y1) in &arms {
        for y in (y0 - 1)..=(y1 + 1) {
            for x in (x0 - 1)..=(x1 + 1) {
                if x < 0 || y < 0 || x >= FW as i32 || y >= FH as i32 { continue; }
                let inside = x >= x0 && x <= x1 && y >= y0 && y <= y1;
                let corner = (x == x0 - 1 || x == x1 + 1) && (y == y0 - 1 || y == y1 + 1);
                if corner { continue; }
                f[y as usize][x as usize] = if inside { if x == x0 { T::Mid } else { T::Light } } else { T::Outline };
            }
        }
    }
    head(&mut f, dir, bob, eyes);
    f
}

// The head is drawn last, over the body, with its own full outline, so it
// never blends into the shoulders.
fn head(f: &mut Frame, dir: Dir, bob: i32, eyes: u8) {
    let (cx, cy, r) = (8.0f32, 6.0 + bob as f32, 4.6f32);
    let inside = |x: i32, y: i32| {
        let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
        dx * dx + dy * dy <= r * r
    };
    let hy = 6 + bob;
    for y in 0..FH as i32 {
        for x in 0..FW as i32 {
            if inside(x, y) {
                // light from the upper right, darker toward the lower left
                let d = (x as f32 - 11.0).powi(2) + (y as f32 - (hy as f32 - 3.0)).powi(2);
                f[y as usize][x as usize] = if d > 75.0 { T::Shadow } else if d > 55.0 { T::Mid } else { T::Light };
            } else if [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| inside(x + dx, y + dy)) {
                f[y as usize][x as usize] = T::Outline;
            }
        }
    }
    let (sx, sy) = (10, (hy - 3) as usize);
    f[sy][sx] = T::Shine; f[sy][sx + 1] = T::Shine; f[sy + 1][sx] = T::Shine;
    // Eyes sit high on the head and show which way the peg faces; none from behind.
    // Eye styles (see EYE_STYLES). Side views show only the near eye.
    let ey = hy;
    let cols: &[i32] = match (dir, eyes) {
        (Dir::Up, _) | (_, 0) => &[],
        (Dir::Down, 4) => &[5, 10],
        (Dir::Down, 7) => &[5, 9],
        (Dir::Down, _) => &[6, 9],
        (Dir::Right, 7) => &[9],
        (Dir::Right, _) => &[10],
        (Dir::Left, 7) => &[4],
        (Dir::Left, _) => &[5],
    };
    let mut put = |x: i32, y: i32, t: T| {
        if x >= 0 && y >= 0 && x < FW as i32 && y < FH as i32 { f[y as usize][x as usize] = t; }
    };
    for &x in cols {
        match eyes {
            2 => { put(x, ey - 1, T::Eye); put(x, ey, T::Eye); }           // tall
            3 => put(x, ey + 1, T::Eye),                                    // lower
            5 => { put(x, ey - 1, T::Glint); put(x, ey, T::Eye); }         // shiny
            7 => { put(x, ey, T::Eye); put(x + 1, ey - 1, T::Eye); put(x + 2, ey, T::Eye); } // happy ^
            _ => put(x, ey, T::Eye),                                        // dots
        }
    }
    if eyes == 6 {                                                          // dots + smile
        match dir {
            Dir::Down => { put(7, ey + 2, T::Eye); put(8, ey + 2, T::Eye); }
            Dir::Right => put(11, ey + 2, T::Eye),
            Dir::Left => put(4, ey + 2, T::Eye),
            Dir::Up => {}
        }
    }
}

fn shade(f: &mut Frame) {
    let solid = |f: &Frame, x: i32, y: i32| {
        x >= 0 && y >= 0 && x < FW as i32 && y < FH as i32 && f[y as usize][x as usize] != T::Empty
    };
    let src = *f;
    // Light comes from the upper right: left and lower edges darken.
    for y in 0..FH as i32 {
        for x in 0..FW as i32 {
            if !solid(&src, x, y) { continue; }
            let mut t = T::Light;
            if !solid(&src, x - 1, y) || !solid(&src, x, y + 1) { t = T::Mid; }
            if !solid(&src, x - 1, y) && !solid(&src, x, y + 1) { t = T::Shadow; }
            if y >= 20 { t = T::Shadow; } // feet
            f[y as usize][x as usize] = t;
        }
    }
    // Outline: every empty pixel touching the figure.
    let filled = *f;
    for y in 0..FH as i32 {
        for x in 0..FW as i32 {
            if solid(&filled, x, y) { continue; }
            let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| solid(&filled, x + dx, y + dy));
            if near { f[y as usize][x as usize] = T::Outline; }
        }
    }
}

fn rgb(c: u32) -> [f32; 3] {
    [((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32]
}

fn toward(c: [f32; 3], to: f32, t: f32) -> [u8; 4] {
    let m = |v: f32| (v + (to - v) * t).round() as u8;
    [m(c[0]), m(c[1]), m(c[2]), 255]
}

fn palette(color: u32, t: T) -> [u8; 4] {
    let c = rgb(color);
    match t {
        T::Empty => [0, 0, 0, 0],
        T::Outline => toward(c, 0.0, 0.65),
        T::Shadow => toward(c, 0.0, 0.3),
        T::Mid => toward(c, 0.0, 0.12),
        T::Light => toward(c, 255.0, 0.08),
        T::Shine => toward(c, 255.0, 0.7),
        T::Eye => [0x1f, 0x29, 0x33, 255],
        T::Glint => [255, 255, 255, 255],
    }
}

fn save(path: &str, w: usize, h: usize, px: &[u8]) {
    let file = BufWriter::new(File::create(path).expect("create png"));
    let mut enc = png::Encoder::new(file, w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(px).unwrap();
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let dirs = [Dir::Down, Dir::Left, Dir::Right, Dir::Up];
    if std::env::args().nth(2).as_deref() == Some("variants") { variants(&out); return; }
    let frames: Vec<Vec<Frame>> = dirs.iter().map(|&d| (0..FRAMES).map(|s| frame(d, s, EYES)).collect()).collect();

    let (w, h) = (FW * FRAMES, FH * DIRS * COLORS.len());
    let mut sheet = vec![0u8; w * h * 4];
    for (ci, &color) in COLORS.iter().enumerate() {
        for (di, row) in frames.iter().enumerate() {
            for (si, fr) in row.iter().enumerate() {
                for y in 0..FH {
                    for x in 0..FW {
                        let (px, py) = (si * FW + x, (ci * DIRS + di) * FH + y);
                        let i = (py * w + px) * 4;
                        sheet[i..i + 4].copy_from_slice(&palette(color, fr[y][x]));
                    }
                }
            }
        }
    }
    save(&format!("{out}/pegs.png"), w, h, &sheet);

    // Preview: colors side by side, 6x, on white.
    let k = 6;
    let (pw, ph) = (w * COLORS.len() * k, FH * DIRS * k);
    let mut prev = vec![255u8; pw * ph * 4];
    for ci in 0..COLORS.len() {
        for y in 0..FH * DIRS {
            for x in 0..w {
                let i = (((ci * DIRS * FH) + y) * w + x) * 4;
                if sheet[i + 3] == 0 { continue; }
                for yy in 0..k {
                    for xx in 0..k {
                        let j = (((y * k + yy) * pw) + (ci * w + x) * k + xx) * 4;
                        prev[j..j + 4].copy_from_slice(&sheet[i..i + 4]);
                    }
                }
            }
        }
    }
    save(&format!("{out}/pegs-preview.png"), pw, ph, &prev);

    // Portraits for the dashboard: head and shoulders of the front standing
    // frame, 16x16, one per color (face-<i>.png).
    for (ci, &color) in COLORS.iter().enumerate() {
        let fr = &frames[0][0];
        let mut px = vec![0u8; 16 * 16 * 4];
        for y in 0..16 { for x in 0..16 {
            let i = (y * 16 + x) * 4;
            px[i..i + 4].copy_from_slice(&palette(color, fr[y + 1][x]));
        }}
        save(&format!("{out}/face-{ci}.png"), 16, 16, &px);
    }
    println!("wrote {out}/pegs.png ({w}x{h}) and pegs-preview.png");
}

// Every eye style side by side: one row per style, showing front, side and
// front-walking in three colors, 8x.
fn variants(out: &str) {
    let k = 8;
    let picks = [(Dir::Down, 0), (Dir::Right, 0), (Dir::Down, 1)];
    let colors = [COLORS[3], COLORS[0], COLORS[7]];
    let cols = picks.len() * colors.len();
    let (w, h) = (cols * (FW + 2) * k, 8 * (FH + 2) * k);
    let mut px = vec![255u8; w * h * 4];
    for style in 0..8u8 {
        for (ci, &color) in colors.iter().enumerate() {
            for (pi, &(d, st)) in picks.iter().enumerate() {
                let fr = frame(d, st, style);
                let (ox, oy) = ((ci * picks.len() + pi) * (FW + 2) + 1, style as usize * (FH + 2) + 1);
                for y in 0..FH { for x in 0..FW {
                    let c = palette(color, fr[y][x]);
                    if c[3] == 0 { continue; }
                    for yy in 0..k { for xx in 0..k {
                        let i = (((oy + y) * k + yy) * w + (ox + x) * k + xx) * 4;
                        px[i..i + 4].copy_from_slice(&c);
                    }}
                }}
            }
        }
    }
    save(&format!("{out}/pegs-eyes.png"), w, h, &px);
    println!("wrote {out}/pegs-eyes.png");
}
