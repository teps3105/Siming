//! GDD 2.2 步驟 1、4：自製三維梯度雜訊（Perlin 2002 改良版，置換表）、fBm、脊狀多重碎形與兩層 domain warp，全部在單位球上取樣（無接縫、極區不擠壓）。
//!
//! 只用 IEEE 754 基本運算與 `floor`（精確運算），種子取自 `worldgen` 子流（GDD 6.5）。
//! 頻率以「赤道上的格數」表示：單位球上的座標乘以 `r_cells / 特徵格數`。

use crate::params::*;
use rand::RngCore;
use rand_chacha::ChaCha8Rng;

/// Perlin 的 12 個梯度方向（立方體稜中點），再重複 4 個湊成 16 個以便取餘。
const GRAD: [[f32; 3]; 16] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [0.0, -1.0, 1.0],
    [0.0, -1.0, -1.0],
];

/// 一個三維梯度雜訊場（Perlin 2002 改良版：置換表 + 12 個稜中點梯度 + 五次平滑）；置換表由 `worldgen` 子流洗牌。
#[derive(Clone, Debug)]
pub struct Noise {
    /// 置換表（512 項，後半重複前半以免取餘）。
    perm: Box<[u8; 512]>,
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Noise {
    pub fn new(rng: &mut ChaCha8Rng) -> Self {
        let mut p = [0u8; 512];
        for (i, v) in p.iter_mut().take(256).enumerate() {
            *v = i as u8;
        }
        // Fisher–Yates，固定順序。
        for i in (1..256usize).rev() {
            let j = (rng.next_u32() as usize) % (i + 1);
            p.swap(i, j);
        }
        for i in 0..256 {
            p[256 + i] = p[i];
        }
        Self { perm: Box::new(p) }
    }

    /// 格點 `(ix, iy, iz)`（已含八度偏移）的梯度與偏移向量的內積。
    #[inline]
    fn grad(&self, ix: usize, iy: usize, iz: usize, dx: f32, dy: f32, dz: f32) -> f32 {
        let p = &self.perm;
        let h = p[p[p[ix & 255] as usize + (iy & 255)] as usize + (iz & 255)];
        let g = GRAD[(h & 15) as usize];
        g[0] * dx + g[1] * dy + g[2] * dz
    }

    /// 單八度三維梯度雜訊，約 −1..1。
    pub fn sample(&self, p: [f32; 3], oct: u32) -> f32 {
        let f = [p[0].floor(), p[1].floor(), p[2].floor()];
        // 以 2^20 平移使索引為非負；每個八度再平移 57 格，使各八度的格點圖樣不同。
        let off = (1 << 20) + oct as i32 * 57;
        let (ix, iy, iz) = ((f[0] as i32 + off) as usize, (f[1] as i32 + off) as usize, (f[2] as i32 + off) as usize);
        let (fx, fy, fz) = (p[0] - f[0], p[1] - f[1], p[2] - f[2]);
        let (ux, uy, uz) = (fade(fx), fade(fy), fade(fz));
        let g = |a: usize, b: usize, c: usize| {
            self.grad(ix + a, iy + b, iz + c, fx - a as f32, fy - b as f32, fz - c as f32)
        };
        let x00 = lerp(g(0, 0, 0), g(1, 0, 0), ux);
        let x10 = lerp(g(0, 1, 0), g(1, 1, 0), ux);
        let x01 = lerp(g(0, 0, 1), g(1, 0, 1), ux);
        let x11 = lerp(g(0, 1, 1), g(1, 1, 1), ux);
        let y0 = lerp(x00, x10, uy);
        let y1 = lerp(x01, x11, uy);
        lerp(y0, y1, uz)
    }

    /// fBm：`octaves` 個八度，`base` 為單位球座標的基頻，間隙度 `WG_LACUNARITY`、增益 `gain`；回傳約 0..1（0.5 為中值）。
    pub fn fbm(&self, p: [f32; 3], base: f32, octaves: u32, gain: f32) -> f32 {
        let mut freq = base;
        let mut amp = 1.0f32;
        let mut sum = 0.0f32;
        let mut norm = 0.0f32;
        for o in 0..octaves {
            sum += amp * self.sample([p[0] * freq, p[1] * freq, p[2] * freq], o);
            norm += amp;
            freq *= WG_LACUNARITY;
            amp *= gain;
        }
        0.5 + 0.5 * sum / norm
    }

    /// 脊狀多重碎形（Musgrave）：每八度取 `(1 − |n|)²`，並以前一八度的訊號加權（山脊上細節多、谷地平滑）；回傳 0..1。
    pub fn ridged(&self, p: [f32; 3], base: f32, octaves: u32, gain: f32) -> f32 {
        let mut freq = base;
        let mut amp = 1.0f32;
        let mut weight = 1.0f32;
        let mut sum = 0.0f32;
        let mut norm = 0.0f32;
        for o in 0..octaves {
            let n = self.sample([p[0] * freq, p[1] * freq, p[2] * freq], o);
            let mut s = 1.0 - (n * WG_RIDGE_SHARPEN).abs().min(1.0);
            s *= s;
            s *= weight;
            weight = (s * WG_RIDGE_WEIGHT).clamp(0.0, 1.0);
            sum += amp * s;
            norm += amp;
            freq *= WG_LACUNARITY;
            amp *= gain;
        }
        sum / norm
    }
}

/// 世界生成使用的全部雜訊場；依固定順序從 `worldgen` 子流取種子。頻率依球半徑（赤道格寬）換算。
#[derive(Clone, Debug)]
pub struct Noises {
    /// 兩層 domain warp，各三個分量。
    warp: [Noise; 6],
    /// 海岸與地貌細節（多八度）。
    detail: Noise,
    /// 大陸輪廓的方向性調變（低頻）。
    shape: Noise,
    /// 島群（中頻）。
    island: Noise,
    /// 造山遮罩（低頻）。
    orogeny: Noise,
    /// 山脈的脊狀多重碎形。
    ridge: Noise,
    /// 丘陵帶遮罩（低頻）。
    hill_mask: Noise,
    /// 丘陵起伏（中頻）。
    hill: Noise,
    /// 溫度擾動。
    temp: Noise,
    /// 濕度擾動。
    humid: Noise,
    /// 副熱帶乾燥帶的緯度擺動。
    dry: Noise,
    /// 球半徑（赤道格寬）。
    r_cells: f32,
}

impl Noises {
    pub fn new(rng: &mut ChaCha8Rng, r_cells: f32) -> Self {
        let mut next = || Noise::new(rng);
        let warp = [next(), next(), next(), next(), next(), next()];
        Self {
            warp,
            detail: next(),
            shape: next(),
            island: next(),
            orogeny: next(),
            ridge: next(),
            hill_mask: next(),
            hill: next(),
            temp: next(),
            humid: next(),
            dry: next(),
            r_cells,
        }
    }

    /// 「特徵格數」換算成單位球座標的頻率。
    #[inline]
    fn freq(&self, cells: f32) -> f32 {
        self.r_cells / cells
    }

    /// GDD 2.2 步驟 1：兩層三維 domain warp（`WG_WARP`：特徵 140、40 格，振幅 40、22 格）。
    pub fn warp(&self, p: [f32; 3]) -> [f32; 3] {
        let f1 = self.freq(WG_WARP1_CELLS);
        let a1 = WG_WARP1_AMP / self.r_cells;
        let q = [p[0] * f1, p[1] * f1, p[2] * f1];
        let p1 = [
            p[0] + self.warp[0].sample(q, 0) * a1,
            p[1] + self.warp[1].sample(q, 0) * a1,
            p[2] + self.warp[2].sample(q, 0) * a1,
        ];
        let f2 = self.freq(WG_WARP2_CELLS);
        let a2 = WG_WARP2_AMP / self.r_cells;
        let q = [p1[0] * f2, p1[1] * f2, p1[2] * f2];
        [
            p1[0] + self.warp[3].sample(q, 0) * a2,
            p1[1] + self.warp[4].sample(q, 0) * a2,
            p1[2] + self.warp[5].sample(q, 0) * a2,
        ]
    }

    /// GDD 2.2 步驟 1：海岸與地貌細節 fBm（`WG_FBM`：特徵 48 格、8 八度、增益 0.5）。
    pub fn detail(&self, p: [f32; 3]) -> f32 {
        self.detail
            .fbm(p, self.freq(WG_FBM_CELLS), WG_FBM_OCTAVES, WG_FBM_DETAIL_GAIN)
    }

    /// GDD 2.2 步驟 1：大陸輪廓調變（`WG_SHAPE`：特徵 70 格、3 八度）。
    pub fn shape(&self, p: [f32; 3]) -> f32 {
        self.shape
            .fbm(p, self.freq(WG_SHAPE_CELLS), WG_SHAPE_OCTAVES, WG_FBM_GAIN)
    }

    /// GDD 2.2 步驟 1：島群雜訊（`WG_ISLAND`：特徵 40 格、4 八度）。
    pub fn island(&self, p: [f32; 3]) -> f32 {
        self.island
            .fbm(p, self.freq(WG_ISLAND_CELLS), WG_ISLAND_OCTAVES, WG_FBM_GAIN)
    }

    /// GDD 2.2 步驟 1：造山雜訊（`WG_OROGENY`：特徵 130 格、3 八度；等值線附近為造山帶）。
    pub fn orogeny(&self, p: [f32; 3]) -> f32 {
        self.orogeny
            .fbm(p, self.freq(WG_OROGENY_CELLS), WG_OROGENY_OCTAVES, WG_FBM_GAIN)
    }

    /// GDD 2.2 步驟 1：脊狀多重碎形（`WG_RIDGE`：特徵 24 格、5 八度）。
    pub fn ridged(&self, p: [f32; 3]) -> f32 {
        self.ridge
            .ridged(p, self.freq(WG_RIDGE_CELLS), WG_RIDGE_OCTAVES, WG_FBM_GAIN)
    }

    /// GDD 2.2 步驟 1：丘陵帶遮罩（`WG_HILL_MASK`：特徵 110 格、3 八度）。
    pub fn hill_mask(&self, p: [f32; 3]) -> f32 {
        self.hill_mask
            .fbm(p, self.freq(WG_HILL_MASK_CELLS), WG_OROGENY_OCTAVES, WG_FBM_GAIN)
    }

    /// GDD 2.2 步驟 1：丘陵起伏（`WG_HILL`：特徵 22 格、3 八度）。
    pub fn hills(&self, p: [f32; 3]) -> f32 {
        self.hill
            .fbm(p, self.freq(WG_HILL_CELLS), WG_HILL_OCTAVES, WG_FBM_GAIN)
    }

    /// GDD 2.2 步驟 4：溫度擾動（`WG_CLIMATE_NOISE`：特徵 110 格、3 八度）。
    pub fn temp(&self, p: [f32; 3]) -> f32 {
        self.temp
            .fbm(p, self.freq(WG_CLIMATE_CELLS), WG_CLIMATE_OCTAVES, WG_FBM_GAIN)
    }

    /// GDD 2.2 步驟 4：濕度擾動（`WG_CLIMATE_NOISE`：特徵 110 格、4 八度）。
    pub fn humid(&self, p: [f32; 3]) -> f32 {
        self.humid.fbm(
            p,
            self.freq(WG_CLIMATE_CELLS),
            WG_CLIMATE_OCTAVES + 1,
            WG_FBM_GAIN,
        )
    }

    /// GDD 2.2 步驟 4：副熱帶乾燥帶的緯度擺動（`WG_DRY_BELT`：特徵 160 格、2 八度）。
    pub fn dry(&self, p: [f32; 3]) -> f32 {
        self.dry.fbm(p, self.freq(WG_DRY_WOBBLE_CELLS), WG_DRY_WOBBLE_OCTAVES, WG_FBM_GAIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn gradient_noise_is_bounded_and_continuous() {
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let n = Noise::new(&mut rng);
        let mut prev = n.sample([0.0, 0.3, 0.7], 0);
        for i in 1..2000 {
            let x = i as f32 * 0.01;
            let v = n.sample([x, 0.3, 0.7], 0);
            assert!((-1.2..=1.2).contains(&v));
            assert!((v - prev).abs() < 0.05, "跳變 {prev} → {v}");
            prev = v;
        }
        let r = n.ridged([0.4, 0.2, 0.9], 1.0, 5, 0.5);
        assert!((0.0..=1.0).contains(&r));
    }
}
