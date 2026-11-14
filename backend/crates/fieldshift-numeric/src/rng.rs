// AI-assisted (OpenBot agent)
const INIT_A: u32 = 0x43b0_d7e5;
const MULT_A: u32 = 0x931e_8875;
const INIT_B: u32 = 0x8b51_f9dd;
const MULT_B: u32 = 0x58f3_8ded;
const MIX_MULT_L: u32 = 0xca01_f9dd;
const MIX_MULT_R: u32 = 0x4973_f715;
const XSHIFT: u32 = 16;
const POOL_SIZE: usize = 4;

fn hashmix(value: u32, hash_const: &mut u32) -> u32 {
    let mut v = value ^ *hash_const;
    *hash_const = hash_const.wrapping_mul(MULT_A);
    v = v.wrapping_mul(*hash_const);
    v ^ (v >> XSHIFT)
}

fn mix(x: u32, y: u32) -> u32 {
    let r = MIX_MULT_L
        .wrapping_mul(x)
        .wrapping_sub(MIX_MULT_R.wrapping_mul(y));
    r ^ (r >> XSHIFT)
}

/// `SeedSequence(seed).generate_state(n_words64, np.uint64)` para semente inteira não negativa.
pub fn seed_sequence_state_u64(seed: u64, n_words64: usize) -> Vec<u64> {
    // _coerce_to_uint32_array: palavras de 32 bits, menos significativa primeiro
    let mut entropy: Vec<u32> = Vec::new();
    let mut s = seed;
    loop {
        entropy.push((s & 0xffff_ffff) as u32);
        s >>= 32;
        if s == 0 {
            break;
        }
    }
    let mut pool = [0u32; POOL_SIZE];
    let mut hc = INIT_A;
    for (i, slot) in pool.iter_mut().enumerate() {
        let v = entropy.get(i).copied().unwrap_or(0);
        *slot = hashmix(v, &mut hc);
    }
    for i_src in 0..POOL_SIZE {
        for i_dst in 0..POOL_SIZE {
            if i_src != i_dst {
                let h = hashmix(pool[i_src], &mut hc);
                pool[i_dst] = mix(pool[i_dst], h);
            }
        }
    }
    for &e in entropy.iter().skip(POOL_SIZE) {
        for slot in pool.iter_mut() {
            let h = hashmix(e, &mut hc);
            *slot = mix(*slot, h);
        }
    }
    let n32 = n_words64 * 2;
    let mut hb = INIT_B;
    let mut words = Vec::with_capacity(n32);
    for i in 0..n32 {
        let mut v = pool[i % POOL_SIZE];
        v ^= hb;
        hb = hb.wrapping_mul(MULT_B);
        v = v.wrapping_mul(hb);
        v ^= v >> XSHIFT;
        words.push(v);
    }
    words
        .chunks(2)
        .map(|c| u64::from(c[0]) | (u64::from(c[1]) << 32))
        .collect()
}

const PCG_MULT: u128 = 0x2360_ed05_1fc6_5da4_4385_df64_9fcc_f645;

/// Gerador PCG64 (XSL-RR 128/64) com o buffer de 32 bits do numpy.
pub struct Pcg64 {
    state: u128,
    inc: u128,
    has_u32: bool,
    u32_buf: u32,
}

impl Pcg64 {
    /// `np.random.default_rng(seed)`.
    pub fn from_seed(seed: u64) -> Self {
        let v = seed_sequence_state_u64(seed, 4);
        let initstate = (u128::from(v[0]) << 64) | u128::from(v[1]);
        let initseq = (u128::from(v[2]) << 64) | u128::from(v[3]);
        let mut g = Pcg64 {
            state: 0,
            inc: (initseq << 1) | 1,
            has_u32: false,
            u32_buf: 0,
        };
        g.step();
        g.state = g.state.wrapping_add(initstate);
        g.step();
        g
    }

    fn step(&mut self) {
        self.state = self.state.wrapping_mul(PCG_MULT).wrapping_add(self.inc);
    }

    pub fn next_u64(&mut self) -> u64 {
        self.step();
        let s = self.state;
        let x = ((s >> 64) as u64) ^ (s as u64);
        let rot = (s >> 122) as u32;
        x.rotate_right(rot)
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.has_u32 {
            self.has_u32 = false;
            return self.u32_buf;
        }
        let n = self.next_u64();
        self.has_u32 = true;
        self.u32_buf = (n >> 32) as u32;
        (n & 0xffff_ffff) as u32
    }

    /// `integers(0, high, n)` com `high - 1 <= u32::MAX` (Lemire com rejeição, 32 bits).
    pub fn integers_below(&mut self, high: u32, n: usize) -> Vec<u32> {
        let rng = high - 1;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            if rng == 0 {
                out.push(0);
                continue;
            }
            let rng_excl = rng.wrapping_add(1);
            let mut m = u64::from(self.next_u32()) * u64::from(rng_excl);
            let mut leftover = (m & 0xffff_ffff) as u32;
            if leftover < rng_excl {
                let threshold = (u32::MAX - rng) % rng_excl;
                while leftover < threshold {
                    m = u64::from(self.next_u32()) * u64::from(rng_excl);
                    leftover = (m & 0xffff_ffff) as u32;
                }
            }
            out.push((m >> 32) as u32);
        }
        out
    }
}

impl Pcg64 {
    /// numpy int64 exclusive bounds; 32-bit Lemire for small ranges, 64-bit otherwise.
    pub fn integers(&mut self, low: i64, high: i64, n: usize) -> Result<Vec<i64>, &'static str> {
        if low >= high {
            return Err("integers requires low < high");
        }
        let width = (i128::from(high) - i128::from(low)) as u64;
        if width <= u64::from(u32::MAX) {
            return Ok(self
                .integers_below(width as u32, n)
                .into_iter()
                .map(|x| (i128::from(low) + i128::from(x)) as i64)
                .collect());
        }
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let v = if width == 1u64 << 32 {
                u64::from(self.next_u32())
            } else {
                let threshold = width.wrapping_neg() % width;
                loop {
                    let m = u128::from(self.next_u64()) * u128::from(width);
                    if m as u64 >= threshold {
                        break (m >> 64) as u64;
                    }
                }
            };
            out.push((i128::from(low) + i128::from(v)) as i64);
        }
        Ok(out)
    }
}
