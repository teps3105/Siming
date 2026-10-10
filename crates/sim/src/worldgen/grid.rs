//! GDD 2.2 世界生成共用的格點輔助：等距圓柱投影的球面格點（x 東西環繞）、鄰格走訪與固定的方向順序
//! （GDD 6.5 規則 3：迭代順序固定）、球面座標與弦長距離、只用基本運算的三角函數（GDD 6.5 規則 4）。

use rand::RngCore;
use rand_chacha::ChaCha8Rng;

/// 八方向，固定順序：北、東、南、西，再東北、東南、西南、西北。前四個為正向。
pub const DIRS8: [(i32, i32); 8] = [
    (0, -1),
    (1, 0),
    (0, 1),
    (-1, 0),
    (1, -1),
    (1, 1),
    (-1, 1),
    (-1, -1),
];

/// 無效索引。
pub const NONE: u32 = u32::MAX;

pub const PI: f32 = std::f32::consts::PI;
pub const TAU: f32 = 2.0 * std::f32::consts::PI;

/// 對角步長（√2；以 `sqrt` 計算，GDD 6.5 規則 4 允許）。
pub fn diag_len() -> f32 {
    2.0f32.sqrt()
}

/// 把 x 環繞到 0..w。
#[inline]
pub fn wrap_x(w: usize, x: i32) -> usize {
    let w = w as i32;
    (((x % w) + w) % w) as usize
}

/// 八鄰格（x 東西環繞；y 不環繞）；回傳 `(索引, 是否為對角)`。
pub fn nb8(w: usize, h: usize, c: usize) -> impl Iterator<Item = (usize, bool)> {
    let x = (c % w) as i32;
    let y = (c / w) as i32;
    DIRS8.iter().filter_map(move |&(dx, dy)| {
        let ny = y + dy;
        if ny < 0 || ny >= h as i32 {
            None
        } else {
            Some((ny as usize * w + wrap_x(w, x + dx), dx != 0 && dy != 0))
        }
    })
}

/// 四鄰格。
pub fn nb4(w: usize, h: usize, c: usize) -> impl Iterator<Item = usize> {
    nb8(w, h, c).filter(|&(_, d)| !d).map(|(i, _)| i)
}

/// 環繞下的格點 Chebyshev 距離（八鄰步數）。
pub fn cheb(w: usize, a: usize, b: usize) -> u32 {
    let dx = ((a % w) as i32 - (b % w) as i32).abs();
    let dx = dx.min(w as i32 - dx);
    let dy = ((a / w) as i32 - (b / w) as i32).abs();
    dx.max(dy) as u32
}

/// U[0,1) 的 f32（24 位元尾數）。
pub fn unit(rng: &mut ChaCha8Rng) -> f32 {
    (rng.next_u32() >> 8) as f32 / 16_777_216.0
}

/// U[a,b)。
pub fn uniform(rng: &mut ChaCha8Rng, a: f32, b: f32) -> f32 {
    a + (b - a) * unit(rng)
}

/// 0..n 的均勻整數（n > 0）。
pub fn below(rng: &mut ChaCha8Rng, n: usize) -> usize {
    (rng.next_u64() % n as u64) as usize
}

/// 非負 f32 的排序鍵（IEEE 754 非負浮點的位元序等於數值序）。
pub fn key(v: f32) -> u32 {
    debug_assert!(v >= 0.0 || v == 0.0);
    v.to_bits()
}

/// 把雜訊場以最小／最大值線性拉伸到完整的 0..1（值雜訊的 fBm 會集中在 0.5 附近；實作細節）。
pub fn stretch(v: &mut [f32]) {
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for &x in v.iter() {
        lo = lo.min(x);
        hi = hi.max(x);
    }
    let span = (hi - lo).max(1e-6);
    for x in v.iter_mut() {
        *x = (*x - lo) / span;
    }
}

/// `|x| ≤ π/2` 的正弦：泰勒展開到 x⁹（只用基本運算，GDD 6.5 規則 4）。
fn sin_half(x: f32) -> f32 {
    let x2 = x * x;
    x * (1.0
        + x2 * (-1.0 / 6.0
            + x2 * (1.0 / 120.0 + x2 * (-1.0 / 5040.0 + x2 * (1.0 / 362_880.0)))))
}

/// `asin(z)` 的度數（|z| ≤ 1；只用基本運算：以 `atan2` 的多項式近似，誤差 < 0.1°）。
pub fn asin_deg(z: f32) -> f32 {
    let z = z.clamp(-1.0, 1.0);
    // asin(z) = atan(z / sqrt(1 − z²))；用 atan 的奇次多項式（|x| ≤ 1）與 atan(x) = π/2 − atan(1/x)（|x| > 1）。
    let x = z / (1.0 - z * z).max(1e-12).sqrt();
    let atan_unit = |t: f32| {
        let t2 = t * t;
        t * (0.999_866 + t2 * (-0.330_299 + t2 * (0.180_141 + t2 * (-0.085_133 + t2 * 0.020_835))))
    };
    let a = if x.abs() <= 1.0 {
        atan_unit(x)
    } else {
        let s = if x > 0.0 { 1.0 } else { -1.0 };
        s * (PI / 2.0) - atan_unit(1.0 / x)
    };
    a * 180.0 / PI
}

/// 正弦（任意角，弧度）：先把角度縮到 [−π, π)，再摺到 [−π/2, π/2]。
pub fn sin(a: f32) -> f32 {
    let mut r = a - TAU * ((a + PI) / TAU).floor();
    if r > PI / 2.0 {
        r = PI - r;
    } else if r < -PI / 2.0 {
        r = -PI - r;
    }
    sin_half(r)
}

/// 餘弦。
pub fn cos(a: f32) -> f32 {
    sin(a + PI / 2.0)
}

/// 球面格點表：每列的緯度與每欄的經度（GDD 2.2 尺寸：等距圓柱投影，W = 2H）。
pub struct Sphere {
    pub w: usize,
    pub h: usize,
    /// 球半徑，以赤道上一格的寬度為單位：W / 2π。
    pub r_cells: f32,
    pub lat: Vec<f32>,
    pub cos_lat: Vec<f32>,
    pub sin_lat: Vec<f32>,
    pub cos_lon: Vec<f32>,
    pub sin_lon: Vec<f32>,
}

/// 列 y 的緯度（弧度；y = 0 為北極側）：`π/2 − π(y + 0.5)/H`。
pub fn row_lat(h: usize, y: usize) -> f32 {
    PI / 2.0 - PI * (y as f32 + 0.5) / h as f32
}

/// 欄 x 的經度（弧度）：`2π(x + 0.5)/W`。
pub fn col_lon(w: usize, x: usize) -> f32 {
    TAU * (x as f32 + 0.5) / w as f32
}

impl Sphere {
    pub fn new(w: usize, h: usize) -> Self {
        Self::with_radius(w, h, w as f32 / TAU)
    }

    /// 以指定的球半徑（距離單位）建立：地理網格比遊戲網格細 `scale` 倍時，仍以遊戲格為距離單位（2.2 尺寸）。
    pub fn with_radius(w: usize, h: usize, r_cells: f32) -> Self {
        let lat: Vec<f32> = (0..h).map(|y| row_lat(h, y)).collect();
        let lon: Vec<f32> = (0..w).map(|x| col_lon(w, x)).collect();
        Self {
            w,
            h,
            r_cells,
            cos_lat: lat.iter().map(|&a| cos(a)).collect(),
            sin_lat: lat.iter().map(|&a| sin(a)).collect(),
            cos_lon: lon.iter().map(|&a| cos(a)).collect(),
            sin_lon: lon.iter().map(|&a| sin(a)).collect(),
            lat,
        }
    }

    /// 格心在單位球上的點。
    #[inline]
    pub fn unit(&self, c: usize) -> [f32; 3] {
        let (x, y) = (c % self.w, c / self.w);
        [
            self.cos_lat[y] * self.cos_lon[x],
            self.cos_lat[y] * self.sin_lon[x],
            self.sin_lat[y],
        ]
    }

    /// 格的面積權重 cos(lat)。
    #[inline]
    pub fn area(&self, c: usize) -> f32 {
        self.cos_lat[c / self.w]
    }

    /// 兩格之間的距離，以赤道格寬為單位（弦長近似大圓弧長；GDD 2.2「距離」）。
    #[inline]
    pub fn dist(&self, a: usize, b: usize) -> f32 {
        chord(self.unit(a), self.unit(b)) * self.r_cells
    }

    /// 距離平方（同上）。
    #[inline]
    pub fn dist2(&self, a: usize, b: usize) -> f32 {
        let d = chord2(self.unit(a), self.unit(b));
        d * self.r_cells * self.r_cells
    }

    /// 以格 `c` 為中心、半徑 `r`（赤道格寬）的外接方框內所有格（x 環繞、y 截斷），固定順序。
    pub fn box_cells(&self, c: usize, r: f32) -> Vec<usize> {
        let (cx, cy) = ((c % self.w) as i32, (c / self.w) as i32);
        let ry = r.ceil() as i32;
        let mut out = Vec::new();
        for dy in -ry..=ry {
            let y = cy + dy;
            if y < 0 || y >= self.h as i32 {
                continue;
            }
            let cl = self.cos_lat[y as usize].max(1e-3);
            let rx = ((r / cl).ceil() as i32).min(self.w as i32 / 2);
            for dx in -rx..=rx {
                out.push(y as usize * self.w + wrap_x(self.w, cx + dx));
            }
        }
        out
    }
}

#[inline]
pub fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
pub fn chord2(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    dot(d, d)
}

#[inline]
pub fn chord(a: [f32; 3], b: [f32; 3]) -> f32 {
    chord2(a, b).sqrt()
}

pub fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = dot(v, v).sqrt().max(1e-9);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// 八鄰連通分量標記（只含 `member` 為真的格）。回傳 `(label 陣列, 各分量格數)`；label 依首格索引升冪編號。
pub fn components(w: usize, h: usize, member: &[bool], eight: bool) -> (Vec<u32>, Vec<u32>) {
    let n = w * h;
    let mut label = vec![NONE; n];
    let mut sizes: Vec<u32> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for s in 0..n {
        if !member[s] || label[s] != NONE {
            continue;
        }
        let id = sizes.len() as u32;
        label[s] = id;
        stack.push(s);
        let mut cnt = 0u32;
        while let Some(c) = stack.pop() {
            cnt += 1;
            if eight {
                for (nb, _) in nb8(w, h, c) {
                    if member[nb] && label[nb] == NONE {
                        label[nb] = id;
                        stack.push(nb);
                    }
                }
            } else {
                for nb in nb4(w, h, c) {
                    if member[nb] && label[nb] == NONE {
                        label[nb] = id;
                        stack.push(nb);
                    }
                }
            }
        }
        sizes.push(cnt);
    }
    (label, sizes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trig_matches_std() {
        let mut a = -20.0f32;
        while a < 20.0 {
            assert!((sin(a) - a.sin()).abs() < 1e-5, "sin {a}");
            assert!((cos(a) - a.cos()).abs() < 1e-5, "cos {a}");
            a += 0.137;
        }
    }

    #[test]
    fn wrap_neighbours() {
        let (w, h) = (8, 4);
        let east: Vec<usize> = nb8(w, h, 7).map(|(i, _)| i).collect();
        assert!(east.contains(&0));
        assert!(east.contains(&8));
        assert_eq!(cheb(w, 0, 7), 1);
    }
}
