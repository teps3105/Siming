//! GDD 5.7「司命案頭」器物繪製模組（7.9 #34）：紙紋、毛邊紙簽、印章、天時儀、命簿卷軸、墨條、朱描焦點，
//! 以及墨暈／壓印／展卷動畫。全部以 egui 向量繪圖與固定種子的程序紙紋繪製（9.3、9.8），不用點陣圖檔。
use bevy_egui::egui::{
    self, epaint, Align2, Color32, FontId, Id, Mesh, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextureId, Ui, Vec2,
};

use super::theme::*;

/// 裱綾（5.1 silk）。
pub const SILK: Color32 = Color32::from_rgb(0x3C, 0x4A, 0x52);
/// 紙簽亮面（5.1 paper-light）。
pub const PAPER_LIGHT: Color32 = Color32::from_rgb(0xF3, 0xEB, 0xD8);
/// 朱印暗部（印緣）。
const SEAL_DARK: Color32 = Color32::from_rgb(0x8E, 0x2A, 0x22);
/// 紙紋貼圖邊長（9.3 `ui_paper`）。
const TILE: f32 = 256.0;

/// 繪圖脈絡：紙紋貼圖與版面縮放（Deck 1.15，桌機 1.0）。
#[derive(Clone, Copy)]
pub struct Ink {
    pub paper: TextureId,
    pub scale: f32,
}

impl Ink {
    pub fn px(&self, v: f32) -> f32 {
        v * self.scale
    }
}

// ── 固定種子雜湊與抖動 ─────────────────────────────

fn hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// `[-1, 1]` 的固定抖動：同一 `(seed, i)` 每幀相同（5.7 原則 4）。
pub fn jit(seed: u64, i: u32) -> f32 {
    ((hash(seed ^ (i as u64).wrapping_mul(0x2545_F491_4F6C_DD1D)) >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
}

fn unit(seed: u64, i: u32) -> f32 {
    jit(seed, i) * 0.5 + 0.5
}

fn seed_of(rect: Rect) -> u64 {
    (rect.min.x.round() as i64 as u64).wrapping_mul(31).wrapping_add(rect.min.y.round() as i64 as u64)
}

// ── 紙紋貼圖（9.3 `ui_paper`） ──────────────────────

/// 啟動時生成的 256×256 平鋪紙紋：低頻明暗斑 + 纖維；以白為底，由頂點色染成紙色。
pub fn paper_tile() -> egui::ColorImage {
    const N: usize = TILE as usize;
    const L: usize = 8;
    let lattice: Vec<f32> = (0..(L * L) as u32).map(|i| jit(0x5157, i)).collect();
    let mut v = vec![0.0f32; N * N];
    for y in 0..N {
        for x in 0..N {
            let fx = x as f32 / N as f32 * L as f32;
            let fy = y as f32 / N as f32 * L as f32;
            let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
            let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
            let s = |t: f32| t * t * (3.0 - 2.0 * t);
            let (tx, ty) = (s(tx), s(ty));
            let g = |i: usize, j: usize| lattice[(j % L) * L + (i % L)];
            let a = g(x0, y0) + (g(x0 + 1, y0) - g(x0, y0)) * tx;
            let b = g(x0, y0 + 1) + (g(x0 + 1, y0 + 1) - g(x0, y0 + 1)) * tx;
            v[y * N + x] = (a + (b - a) * ty) * 0.035 + jit(0x7A9E, (y * N + x) as u32) * 0.018;
        }
    }
    // 纖維：隨機方向的短線段，稍微壓暗
    for k in 0..900u32 {
        let sx = unit(0xF1B, k * 3) * N as f32;
        let sy = unit(0xF1B, k * 3 + 1) * N as f32;
        let ang = unit(0xF1B, k * 3 + 2) * std::f32::consts::TAU;
        let len = 8.0 + unit(0xA3, k) * 22.0;
        let (dx, dy) = (ang.cos(), ang.sin());
        let mut t = 0.0;
        while t < len {
            let x = ((sx + dx * t).rem_euclid(N as f32)) as usize;
            let y = ((sy + dy * t).rem_euclid(N as f32)) as usize;
            v[y * N + x] -= 0.045;
            t += 0.7;
        }
    }
    let pixels = v
        .iter()
        .map(|d| {
            let c = ((1.0 - d.max(0.0) * 1.6 + d.min(0.0) * 0.4) * 255.0).clamp(196.0, 255.0) as u8;
            Color32::from_rgb(c, c, c)
        })
        .collect();
    egui::ColorImage { size: [N, N], pixels }
}

// ── 幾何輔助 ──────────────────────────────────────

/// 扇形三角化填多邊形（星形多邊形皆可）；`paper` 為真時貼紙紋。
pub fn poly_shape(pts: &[Pos2], color: Color32, paper: Option<TextureId>) -> Shape {
    if pts.len() < 3 {
        return Shape::Noop;
    }
    let mut mesh = Mesh::with_texture(paper.unwrap_or_default());
    let c = pts.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / pts.len() as f32;
    let uv = |p: Pos2| if paper.is_some() { Pos2::new(p.x / TILE, p.y / TILE) } else { epaint::WHITE_UV };
    mesh.vertices.push(epaint::Vertex { pos: c.to_pos2(), uv: uv(c.to_pos2()), color });
    for p in pts {
        mesh.vertices.push(epaint::Vertex { pos: *p, uv: uv(*p), color });
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    Shape::mesh(mesh)
}

pub fn fill_poly(painter: &egui::Painter, pts: &[Pos2], color: Color32, paper: Option<TextureId>) {
    painter.add(poly_shape(pts, color, paper));
}

/// 矩形的毛邊輪廓：沿四邊取點並沿法線抖動 `amp` px。
pub fn deckle(rect: Rect, amp: f32, seed: u64) -> Vec<Pos2> {
    let corners = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()];
    let mut pts = Vec::new();
    let mut k = 0u32;
    for e in 0..4 {
        let (a, b) = (corners[e], corners[(e + 1) % 4]);
        let len = (b - a).length();
        let n = ((len / 5.0).ceil() as u32).max(1);
        let normal = Vec2::new(b.y - a.y, -(b.x - a.x)).normalized() * -1.0;
        for i in 0..n {
            let t = i as f32 / n as f32;
            // 角落不抖，避免尖角破形
            let w = (t * (1.0 - t) * 4.0).min(1.0);
            pts.push(a + (b - a) * t + normal * amp * jit(seed, k) * w);
            k += 1;
        }
    }
    pts
}

/// 圓角方形（`r` 圓角）或正圓的輪廓，邊緣抖動 `amp` px。
pub fn seal_points(rect: Rect, round: bool, amp: f32, seed: u64) -> Vec<Pos2> {
    let c = rect.center();
    let n = 40;
    let (hw, hh) = (rect.width() * 0.5, rect.height() * 0.5);
    let r = if round { hw } else { hw * 0.18 };
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let (ca, sa) = (a.cos(), a.sin());
            let p = if round {
                c + Vec2::new(ca, sa) * hw
            } else {
                // 超橢圓近似的圓角方形：先取方形邊界，再向內收圓角
                let sx = ca.signum();
                let sy = sa.signum();
                let bx = c.x + sx * (hw - r) + ca * r;
                let by = c.y + sy * (hh - r) + sa * r;
                let m = ca.abs().max(sa.abs());
                let px = c.x + sx * (hw - r) + (ca / m).clamp(-1.0, 1.0) * r;
                let py = c.y + sy * (hh - r) + (sa / m).clamp(-1.0, 1.0) * r;
                // 兩種近似混合：邊上取方形、角上取圓
                let w = (ca.abs() - sa.abs()).abs();
                Pos2::new(bx + (px - bx) * w, by + (py - by) * w)
            };
            p + Vec2::new(ca, sa) * amp * jit(seed, i as u32)
        })
        .collect()
}

/// 兩端收細的墨筆劃（三角帶網格）。
pub fn brush_rule(painter: &egui::Painter, a: Pos2, b: Pos2, width: f32, color: Color32, seed: u64) {
    let len = (b - a).length();
    if len < 0.5 {
        return;
    }
    let n = ((len / 4.0).ceil() as usize).clamp(6, 64);
    let dir = (b - a) / len;
    let normal = Vec2::new(-dir.y, dir.x);
    let mut mesh = Mesh::default();
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let taper = (t * std::f32::consts::PI).sin().max(0.0).powf(0.35);
        // 低頻的筆壓起伏（相鄰頂點內插，避免鋸齒）
        let k = i as f32 / 6.0;
        let (k0, kf) = (k.floor() as u32, k.fract());
        let wobble = jit(seed, k0) + (jit(seed, k0 + 1) - jit(seed, k0)) * kf;
        let half = width * 0.5 * taper * (1.0 + 0.08 * wobble);
        let p = a + dir * (len * t);
        mesh.colored_vertex(p + normal * half, color);
        mesh.colored_vertex(p - normal * half, color);
        if i > 0 {
            let k = (i * 2) as u32;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k + 1, k);
        }
    }
    painter.add(Shape::mesh(mesh));
}

fn stroke(w: f32, c: Color32) -> Stroke {
    Stroke::new(w, c)
}

fn alpha(c: Color32, a: f32) -> Color32 {
    c.gamma_multiply(a.clamp(0.0, 1.0))
}

// ── 紙簽與面板 ──────────────────────────────────────

/// 柔影：向外擴散的幾層淡墨。
fn soft_shadow(painter: &egui::Painter, rect: Rect, strength: f32) {
    for k in 0..3 {
        let e = 1.5 + k as f32 * 1.5;
        painter.rect_filled(rect.translate(Vec2::new(1.0, 2.0)).expand(e), 4.0, alpha(INK, 0.05 * strength / (k as f32 + 1.0)));
    }
}

/// 毛邊紙簽的形狀（紙紋填充 + 淡墨描邊 + 柔影），供放入背景占位。
pub fn slip_shapes(rect: Rect, ink: &Ink, a: f32) -> Vec<Shape> {
    let seed = seed_of(rect);
    let mut v = Vec::new();
    for k in 0..3 {
        let e = 1.5 + k as f32 * 1.5;
        v.push(Shape::rect_filled(rect.translate(Vec2::new(1.0, 2.0)).expand(e), 4.0, alpha(INK, 0.05 * a / (k as f32 + 1.0))));
    }
    let pts = deckle(rect, 1.6, seed);
    v.push(poly_shape(&pts, alpha(PAPER_LIGHT, a), Some(ink.paper)));
    v.push(Shape::closed_line(pts, stroke(1.0, alpha(INK_LIGHT, 0.55 * a))));
    v
}

/// 毛邊紙簽：直接畫出。
pub fn slip(painter: &egui::Painter, rect: Rect, ink: &Ink, a: f32) {
    painter.add(Shape::Vec(slip_shapes(rect, ink, a)));
}

/// 題簽：帶左側朱記的紙簽，寫一行墨字。回傳佔用矩形。
pub fn title_slip(ui: &mut Ui, ink: &Ink, text: &str, font: f32, a: f32) -> egui::Response {
    let fid = FontId::proportional(font);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), fid.clone(), INK);
    let pad = Vec2::new(ink.px(12.0), ink.px(6.0));
    let size = galley.size() + pad * 2.0 + Vec2::new(ink.px(8.0), 0.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    slip(p, rect, ink, a);
    // 左緣朱記
    let x = rect.left() + ink.px(6.0);
    brush_rule(p, Pos2::new(x, rect.top() + ink.px(5.0)), Pos2::new(x, rect.bottom() - ink.px(5.0)), 2.5, alpha(SEAL_RED, a), seed_of(rect) + 3);
    p.galley(Pos2::new(rect.left() + pad.x + ink.px(8.0), rect.center().y - galley.size().y * 0.5), galley, alpha(INK, a));
    resp
}

/// 停留提示：毛邊紙片，一行墨字，下方細筆橫劃。`anchor` 為提示依附的矩形（置於其上方並置中）；
/// `pointer` 為真時改放在指標右下。淡入 0.15 秒（以 `id` 記住首次出現的時間）。
pub fn tooltip(ctx: &egui::Context, ink: &Ink, id: Id, anchor: Rect, text: &str, pointer: bool) {
    let now = ctx.input(|i| i.time);
    let shown = ctx.data_mut(|d| *d.get_temp_mut_or(id.with("tip_t"), now));
    let a = ((now - shown) / 0.15).clamp(0.0, 1.0) as f32;
    let font = FontId::proportional(ink.px(14.0).max(if ink.scale > 1.0 { 16.0 } else { 0.0 }));
    let galley = ctx.fonts(|f| f.layout_no_wrap(text.to_owned(), font, INK));
    let pad = Vec2::new(ink.px(10.0), ink.px(6.0));
    let size = galley.size() + pad * 2.0;
    let screen = ctx.screen_rect();
    let mut pos = if pointer {
        anchor.min + Vec2::new(14.0, 18.0)
    } else {
        Pos2::new(anchor.center().x - size.x * 0.5, anchor.top() - size.y - ink.px(8.0))
    };
    pos.x = pos.x.clamp(screen.left() + 4.0, screen.right() - size.x - 4.0);
    pos.y = pos.y.clamp(screen.top() + 4.0, screen.bottom() - size.y - 4.0);
    let rect = Rect::from_min_size(pos, size);
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, id));
    slip(&p, rect, ink, a);
    p.galley(rect.min + pad, galley, alpha(INK, a));
    let y = rect.bottom() - ink.px(4.0);
    brush_rule(&p, Pos2::new(rect.left() + pad.x, y), Pos2::new(rect.right() - pad.x, y), 1.6, alpha(INK_LIGHT, 0.8 * a), seed_of(rect));
}

/// 元件是否為搖桿焦點。不用 `Response::has_focus`：它另外要求視窗有鍵盤焦點，
/// 在無視窗管理員的 Xvfb 驗收環境下恆為假；介面焦點只看 egui 記憶即可。
pub fn focused(ctx: &egui::Context, resp: &egui::Response) -> bool {
    ctx.memory(|m| m.has_focus(resp.id))
}

/// 對一個元件顯示停留／焦點提示（鍵鼠停留或搖桿焦點時）。
pub fn hover_tip(ctx: &egui::Context, ink: &Ink, resp: &egui::Response, text: &str) {
    if resp.hovered() || focused(ctx, resp) {
        tooltip(ctx, ink, resp.id.with("tip"), resp.rect, text, false);
    } else {
        ctx.data_mut(|d| d.remove_temp::<f64>(resp.id.with("tip").with("tip_t")));
    }
}

// ── 朱描焦點（5.9） ───────────────────────────────

/// 搖桿焦點：朱紅毛筆描邊，略帶抖動。
pub fn focus_ring(painter: &egui::Painter, rect: Rect) {
    let r = rect.expand(4.0);
    let pts = seal_points(r, false, 1.2, seed_of(r) ^ 0xF0C5);
    painter.add(Shape::closed_line(pts.clone(), stroke(4.0, alpha(SEAL_RED, 0.28))));
    painter.add(Shape::closed_line(pts, stroke(1.8, SEAL_RED)));
}

// ── 印章（9.8） ───────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SealState {
    /// 朱底紙字
    Active,
    /// 紙底朱字
    Idle,
    /// 淡墨
    Disabled,
}

/// 印章的互動狀態，由 [`seal`] 或自訂互動（例如天時儀盤心）計算。
#[derive(Clone, Copy, Default)]
pub struct SealFx {
    /// 墨暈 0..1（停留或焦點，0.15 秒）
    pub hover: f32,
    /// 按住中
    pub pressed: bool,
    /// 壓印餘暈 0..1（放開後 0.3 秒淡出）
    pub glow: f32,
    pub focused: bool,
}

/// 由回應計算印章的回饋量（墨暈、壓印、餘暈）。
pub fn seal_fx(ctx: &egui::Context, resp: &egui::Response) -> SealFx {
    let id = resp.id;
    let hover = ctx.animate_bool_with_time(id.with("hov"), resp.hovered() || focused(ctx, resp), 0.15);
    let now = ctx.input(|i| i.time);
    if resp.clicked() {
        ctx.data_mut(|d| d.insert_temp(id.with("press"), now));
    }
    let since = now - ctx.data(|d| d.get_temp::<f64>(id.with("press"))).unwrap_or(-10.0);
    let glow = (1.0 - since / 0.3).clamp(0.0, 1.0) as f32;
    SealFx { hover, pressed: resp.is_pointer_button_down_on(), glow, focused: focused(ctx, resp) }
}

/// 在 `rect` 內畫一枚印章（含墨暈、壓印、餘暈、焦點）。
pub fn paint_seal(painter: &egui::Painter, rect: Rect, ch: &str, round: bool, state: SealState, fx: SealFx) {
    let c = rect.center();
    let size = rect.width().min(rect.height());
    let seed = seed_of(rect) ^ (ch.len() as u64);
    // 墨暈
    if fx.hover > 0.01 && state != SealState::Disabled {
        for k in 0..3 {
            let rr = size * (0.56 + 0.1 * k as f32) * (0.85 + 0.15 * fx.hover);
            painter.circle_filled(c, rr, alpha(SEAL_RED, 0.16 * fx.hover / (k as f32 + 1.0)));
        }
    }
    // 壓印餘暈：一圈朱痕向外淡出
    if fx.glow > 0.01 {
        let rr = size * 0.6 * (1.0 + (1.0 - fx.glow) * 0.45);
        painter.circle_stroke(c, rr, stroke(2.0 * fx.glow, alpha(SEAL_RED, 0.6 * fx.glow)));
    }
    let scale = if fx.pressed { 0.94 } else { 1.0 - 0.03 * fx.glow };
    let body = Rect::from_center_size(c, Vec2::splat(size * scale));
    let pts = seal_points(body, round, 0.8, seed);
    let (fill, line, text_col) = match state {
        SealState::Active => (SEAL_RED, stroke(1.2, SEAL_DARK), PAPER_LIGHT),
        SealState::Idle => (alpha(PAPER_LIGHT, 0.96), stroke(1.0, alpha(INK_LIGHT, 0.75)), SEAL_RED),
        SealState::Disabled => (alpha(PAPER_DARK, 0.55), stroke(1.0, alpha(INK_LIGHT, 0.35)), alpha(INK_LIGHT, 0.5)),
    };
    if state != SealState::Disabled {
        soft_shadow(painter, body, if state == SealState::Active { 0.9 } else { 0.5 });
    }
    fill_poly(painter, &pts, fill, None);
    painter.add(Shape::closed_line(pts, line));
    // 飛白缺口：朱底時在隨機一角露出紙色
    if state == SealState::Active {
        let k = (unit(seed, 9) * 4.0) as usize;
        let corner = [body.left_top(), body.right_top(), body.right_bottom(), body.left_bottom()][k.min(3)];
        let inward = (c - corner).normalized();
        painter.circle_filled(corner + inward * size * 0.07, size * 0.085, alpha(PAPER_LIGHT, 0.9));
    }
    painter.text(c + Vec2::new(0.0, size * 0.02), Align2::CENTER_CENTER, ch, FontId::proportional(size * 0.58 * scale), text_col);
    if fx.focused {
        focus_ring(painter, rect);
    }
}

/// 印章按鈕：圓角方印（`round` 為假）或圓印。未開放者不可按。
pub fn seal(ui: &mut Ui, ch: &str, round: bool, state: SealState, size: f32) -> egui::Response {
    let sense = if state == SealState::Disabled { Sense::hover() } else { Sense::click() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), sense);
    let fx = seal_fx(ui.ctx(), &resp);
    paint_seal(ui.painter(), rect, ch, round, state, fx);
    resp
}

// ── 墨條與分節（5.8） ─────────────────────────────

/// 墨條：淡墨底線 + 兩端收細的墨筆劃填充；左標籤、右數值。
pub fn ink_meter(ui: &mut Ui, ink: &Ink, label: &str, frac: f32, color: Color32, value: &str) {
    let h = ink.px(22.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let p = ui.painter();
    let font = FontId::proportional(ink.px(14.0).max(if ink.scale > 1.0 { 16.0 } else { 0.0 }));
    let lw = ink.px(50.0);
    let vw = ink.px(64.0);
    p.text(Pos2::new(rect.left(), rect.center().y), Align2::LEFT_CENTER, label, font.clone(), INK_LIGHT);
    p.text(Pos2::new(rect.right(), rect.center().y), Align2::RIGHT_CENTER, value, font, INK);
    let (x0, x1) = (rect.left() + lw, rect.right() - vw);
    let y = rect.center().y + ink.px(1.0);
    p.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], stroke(1.0, alpha(INK_LIGHT, 0.45)));
    let f = frac.clamp(0.0, 1.0);
    if f > 0.005 {
        brush_rule(p, Pos2::new(x0, y), Pos2::new(x0 + (x1 - x0) * f, y), h * 0.42, color, seed_of(rect));
    }
}

/// 分節：墨點 + 節名 + 細線。
pub fn section(ui: &mut Ui, ink: &Ink, title: &str) {
    let h = ink.px(20.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let p = ui.painter();
    let c = Pos2::new(rect.left() + ink.px(4.0), rect.center().y);
    let d = ink.px(3.5);
    p.add(Shape::convex_polygon(
        vec![c + Vec2::new(0.0, -d), c + Vec2::new(d, 0.0), c + Vec2::new(0.0, d), c + Vec2::new(-d, 0.0)],
        INK_LIGHT,
        Stroke::NONE,
    ));
    let font = FontId::proportional(ink.px(13.0).max(if ink.scale > 1.0 { 16.0 } else { 0.0 }));
    let galley = p.layout_no_wrap(title.to_owned(), font, INK_LIGHT);
    let tx = c.x + ink.px(10.0);
    p.galley(Pos2::new(tx, c.y - galley.size().y * 0.5), galley.clone(), INK_LIGHT);
    let x0 = tx + galley.size().x + ink.px(8.0);
    if rect.right() - x0 > 10.0 {
        p.line_segment([Pos2::new(x0, c.y), Pos2::new(rect.right(), c.y)], stroke(1.0, alpha(INK_LIGHT, 0.35)));
    }
}

/// 標題下的筆劃橫線。
pub fn title_rule(ui: &mut Ui, ink: &Ink) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ink.px(8.0)), Sense::hover());
    let y = rect.center().y;
    brush_rule(ui.painter(), Pos2::new(rect.left(), y), Pos2::new(rect.right(), y), ink.px(2.4), INK, seed_of(rect));
}

// ── 命簿卷軸（5.8） ───────────────────────────────

/// 卷軸軸頭：墨色圓柱，兩端略粗。
fn roller(painter: &egui::Painter, rect: Rect) {
    let r = rect.height() * 0.5;
    painter.rect_filled(rect.translate(Vec2::new(0.0, 2.0)), r, alpha(INK, 0.25));
    painter.rect_filled(rect, r, INK);
    let hl = Rect::from_min_max(Pos2::new(rect.left() + r, rect.top() + r * 0.45), Pos2::new(rect.right() - r, rect.top() + r * 0.8));
    painter.rect_filled(hl, r * 0.3, alpha(PAPER, 0.22));
    for x in [rect.left() + r * 0.6, rect.right() - r * 0.6] {
        painter.circle_filled(Pos2::new(x, rect.center().y), r * 1.15, SILK);
        painter.circle_stroke(Pos2::new(x, rect.center().y), r * 1.15, stroke(1.0, INK));
    }
}

/// 命簿完全展開時的總高度（依前一幀記住的內容高度；供定位用）。
pub fn scroll_height(ctx: &egui::Context, id: Id, ink: &Ink) -> f32 {
    let h = ctx.data(|d| d.get_temp::<f32>(id.with("h"))).unwrap_or(ink.px(180.0));
    h + ink.px(14.0) * 2.0 + ink.px(6.0) * 2.0 + ink.px(12.0) * 2.0
}

/// 命簿：上下軸頭、裱綾、紙紋內頁；`t` 為展開量 0..1（0.25 秒展卷、0.18 秒捲回，由呼叫者以
/// `animate_bool_with_time` 計算）。內容由上而下隨展開露出；回傳整個卷軸的矩形。
pub fn scroll_body(ui: &mut Ui, ink: &Ink, id: Id, t: f32, width: f32, content: impl FnOnce(&mut Ui)) -> Rect {
    let silk = ink.px(6.0);
    let roller_h = ink.px(14.0);
    let overhang = ink.px(10.0);
    let pad = ink.px(12.0);
    let top_left = ui.cursor().min;
    let body_left = top_left.x + overhang;
    let body_w = width - overhang * 2.0;
    let prev_h = ui.data(|d| d.get_temp::<f32>(id.with("h"))).unwrap_or(ink.px(180.0));
    let chrome = roller_h * 2.0 + silk * 2.0 + pad * 2.0;
    let edge_y = top_left.y + roller_h + (chrome - roller_h * 2.0 + prev_h) * t;
    let bg_idx = ui.painter().add(Shape::Noop);

    let content_rect = Rect::from_min_size(
        Pos2::new(body_left + silk + pad, top_left.y + roller_h + silk + pad),
        Vec2::new(body_w - (silk + pad) * 2.0, f32::INFINITY),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(content_rect).layout(egui::Layout::top_down(egui::Align::Min)));
    let clip = Rect::from_min_max(top_left, Pos2::new(top_left.x + width, edge_y)).intersect(ui.clip_rect());
    child.set_clip_rect(clip);
    content(&mut child);
    let h = child.min_rect().height().max(ink.px(40.0));
    ui.data_mut(|d| d.insert_temp(id.with("h"), h));

    let total = chrome + h;
    let full = Rect::from_min_size(top_left, Vec2::new(width, total));
    let edge_y = top_left.y + roller_h + (total - roller_h * 2.0) * t;
    // 背景（置於內容之後）
    let body = Rect::from_min_max(Pos2::new(body_left, top_left.y + roller_h * 0.5), Pos2::new(body_left + body_w, edge_y + roller_h * 0.5));
    let mut shapes = Vec::new();
    for k in 0..3 {
        let e = 2.0 + k as f32 * 2.0;
        shapes.push(Shape::rect_filled(body.translate(Vec2::new(2.0, 3.0)).expand(e), 4.0, alpha(INK, 0.06 / (k as f32 + 1.0))));
    }
    shapes.push(Shape::rect_filled(body, 2.0, SILK));
    // 隔水：綾邊內的一道金線
    shapes.push(Shape::rect_stroke(body.shrink(silk * 0.5), 0.0, stroke(1.0, alpha(GOLD, 0.55)), StrokeKind::Middle));
    let paper_rect = body.shrink(silk);
    let mut mesh = Mesh::with_texture(ink.paper);
    let uv = |p: Pos2| Pos2::new(p.x / TILE, p.y / TILE);
    for p in [paper_rect.left_top(), paper_rect.right_top(), paper_rect.right_bottom(), paper_rect.left_bottom()] {
        mesh.vertices.push(epaint::Vertex { pos: p, uv: uv(p), color: PAPER_LIGHT });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    shapes.push(Shape::mesh(mesh));
    shapes.push(Shape::rect_stroke(paper_rect, 0.0, stroke(1.0, alpha(INK_LIGHT, 0.5)), StrokeKind::Inside));
    ui.painter().set(bg_idx, Shape::Vec(shapes));
    // 軸頭（置於內容之上）
    let p = ui.painter();
    roller(p, Rect::from_min_size(top_left, Vec2::new(width, roller_h)));
    roller(p, Rect::from_min_size(Pos2::new(top_left.x, edge_y), Vec2::new(width, roller_h)));
    ui.allocate_rect(full, Sense::hover());
    full
}

// ── 天時儀（5.7、9.8） ─────────────────────────────

/// 天時儀的互動結果。
#[derive(Default)]
pub struct DialOut {
    pub toggle_pause: bool,
    pub set_speed: Option<usize>,
}

/// 天時儀：圓盤 + 盤心行／止圓印 + 盤右四道疾徐墨筆。`month` 為本月（0–11），`speed_idx` 為目前速度階（0–3）。
/// `labels` 為四階的提示文字；`pause_tip` 為盤心提示。
#[allow(clippy::too_many_arguments)]
pub fn time_dial(
    ui: &mut Ui,
    ink: &Ink,
    month: u32,
    paused: bool,
    speed_idx: usize,
    run_ch: (&str, &str),
    pause_tip: &str,
    speed_tips: &[String; 4],
    speed_label: &str,
) -> DialOut {
    let mut out = DialOut::default();
    let d = ink.px(56.0);
    let gap = ink.px(8.0);
    let strokes_w = ink.px(60.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(d + gap + strokes_w, d), Sense::hover());
    let ctx = ui.ctx().clone();
    let p = ui.painter();
    let c = Pos2::new(rect.left() + d * 0.5, rect.center().y);
    let r = d * 0.5;
    let now = ctx.input(|i| i.time) as f32;
    // 盤面
    soft_shadow(p, Rect::from_center_size(c, Vec2::splat(d * 0.9)), 1.0);
    let disc = seal_points(Rect::from_center_size(c, Vec2::splat(d)), true, 0.6, seed_of(rect));
    fill_poly(p, &disc, alpha(PAPER_LIGHT, 0.96), Some(ink.paper));
    p.add(Shape::closed_line(disc, stroke(1.5, INK)));
    p.circle_stroke(c, r * 0.76, stroke(1.0, alpha(INK_LIGHT, 0.7)));
    // 十二月刻度與朱針
    for m in 0..12u32 {
        let a = -std::f32::consts::FRAC_PI_2 + m as f32 * std::f32::consts::TAU / 12.0;
        let dir = Vec2::new(a.cos(), a.sin());
        let (r0, r1, w, col) = if m == month { (r * 0.74, r * 0.95, 2.2, SEAL_RED) } else { (r * 0.82, r * 0.93, 1.0, INK_LIGHT) };
        p.line_segment([c + dir * r0, c + dir * r1], stroke(w, col));
    }
    let a = -std::f32::consts::FRAC_PI_2 + month as f32 * std::f32::consts::TAU / 12.0;
    let dir = Vec2::new(a.cos(), a.sin());
    brush_rule(p, c, c + dir * r * 0.78, 3.0, if paused { INK_LIGHT } else { SEAL_RED }, seed_of(rect) + 1);
    // 運行脈動
    if !paused {
        let pulse = 0.5 + 0.5 * (now * 2.2).sin();
        p.circle_stroke(c, r * 0.5 + pulse * ink.px(5.0), stroke(1.5, alpha(SEAL_RED, 0.35 * (1.0 - pulse) + 0.08)));
    }
    // 盤心印
    let seal_rect = Rect::from_center_size(c, Vec2::splat(r * 0.98));
    let resp = ui.interact(seal_rect, ui.id().with("dial_pp"), Sense::click());
    let fx = seal_fx(&ctx, &resp);
    let (ch, state) = if paused { (run_ch.1, SealState::Idle) } else { (run_ch.0, SealState::Active) };
    paint_seal(ui.painter(), seal_rect, ch, true, state, fx);
    if resp.clicked() {
        out.toggle_pause = true;
    }
    hover_tip(&ctx, ink, &resp, pause_tip);
    // 疾徐四筆
    let x0 = rect.left() + d + gap;
    let label_h = ink.px(14.0).max(if ink.scale > 1.0 { 16.0 } else { 0.0 });
    let base_y = rect.bottom() - label_h - ink.px(2.0);
    let sw = strokes_w / 4.0;
    for (i, tip) in speed_tips.iter().enumerate() {
        let hr = Rect::from_min_max(Pos2::new(x0 + sw * i as f32, rect.top()), Pos2::new(x0 + sw * (i as f32 + 1.0), base_y));
        let resp = ui.interact(hr, ui.id().with(("dial_spd", i)), Sense::click());
        let fx = seal_fx(&ctx, &resp);
        let p = ui.painter();
        let cx = hr.center().x;
        let h = (base_y - rect.top()) * (0.42 + 0.19 * i as f32);
        let active = !paused && i == speed_idx;
        let reached = !paused && i < speed_idx;
        let col = if active { SEAL_RED } else if reached { INK } else { alpha(INK_LIGHT, 0.45) };
        if fx.hover > 0.01 {
            p.circle_filled(Pos2::new(cx, base_y - h * 0.5), sw * 0.9 * fx.hover, alpha(SEAL_RED, 0.14 * fx.hover));
        }
        let w = (if fx.pressed { 0.85 } else { 1.0 }) * sw * 0.42 * (1.0 + 0.35 * fx.glow);
        brush_rule(p, Pos2::new(cx, base_y), Pos2::new(cx, base_y - h), w, col, seed_of(hr));
        if active {
            p.circle_filled(Pos2::new(cx, base_y + ink.px(3.0)), ink.px(2.0), SEAL_RED);
        }
        if fx.focused {
            focus_ring(p, hr);
        }
        if resp.clicked() {
            out.set_speed = Some(i);
        }
        hover_tip(&ctx, ink, &resp, tip);
    }
    ui.painter().text(
        Pos2::new(x0 + strokes_w * 0.5, rect.bottom()),
        Align2::CENTER_BOTTOM,
        speed_label,
        FontId::proportional(label_h),
        if paused { alpha(INK_LIGHT, 0.9) } else { INK },
    );
    out
}

/// 圖例色塊：墨點（略帶毛邊的圓）。
pub fn swatch(ui: &mut Ui, ink: &Ink, color: Color32) {
    let s = ink.px(14.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(s), Sense::hover());
    let pts = seal_points(rect.shrink(1.0), true, 0.9, seed_of(rect));
    fill_poly(ui.painter(), &pts, color, None);
    ui.painter().add(Shape::closed_line(pts, stroke(1.0, alpha(INK_LIGHT, 0.8))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_is_deterministic_and_bounded() {
        for i in 0..200 {
            let a = jit(42, i);
            assert_eq!(a, jit(42, i));
            assert!((-1.0..=1.0).contains(&a));
        }
        assert_ne!(jit(1, 0), jit(2, 0));
    }

    #[test]
    fn paper_tile_is_light_and_tileable_size() {
        let img = paper_tile();
        assert_eq!(img.size, [256, 256]);
        let mean = img.pixels.iter().map(|c| c.r() as f32).sum::<f32>() / img.pixels.len() as f32;
        assert!(mean > 225.0 && mean <= 255.0, "mean {mean}");
        assert!(img.pixels.iter().any(|c| c.r() < 250));
    }

    #[test]
    fn deckle_keeps_corners() {
        let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 40.0));
        let pts = deckle(r, 2.0, 7);
        assert!(pts.len() > 8);
        assert_eq!(pts[0], r.left_top());
        assert!(pts.iter().all(|p| p.x >= -2.5 && p.x <= 102.5 && p.y >= -2.5 && p.y <= 42.5));
    }

    #[test]
    fn seal_points_round_stay_on_circle() {
        let r = Rect::from_center_size(Pos2::new(50.0, 50.0), Vec2::splat(40.0));
        for p in seal_points(r, true, 0.0, 1) {
            assert!(((p - Pos2::new(50.0, 50.0)).length() - 20.0).abs() < 1e-3);
        }
        for p in seal_points(r, false, 0.0, 1) {
            assert!(p.x >= 29.9 && p.x <= 70.1 && p.y >= 29.9 && p.y <= 70.1);
        }
    }
}
