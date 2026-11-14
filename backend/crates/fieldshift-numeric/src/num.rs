// AI-assisted (OpenBot agent)
/// `math.fsum` (CPython): soma exata de Shewchuk com o ajuste final de arredondamento par.
pub fn fsum(xs: &[f64]) -> f64 {
    let mut partials: Vec<f64> = Vec::new();
    for &x0 in xs {
        let mut x = x0;
        let mut i = 0;
        for j in 0..partials.len() {
            let mut y = partials[j];
            if x.abs() < y.abs() {
                std::mem::swap(&mut x, &mut y);
            }
            let hi = x + y;
            let yr = hi - x;
            let lo = y - yr;
            if lo != 0.0 {
                partials[i] = lo;
                i += 1;
            }
            x = hi;
        }
        partials.truncate(i);
        partials.push(x);
    }
    let mut n = partials.len();
    if n == 0 {
        return 0.0;
    }
    n -= 1;
    let mut hi = partials[n];
    let mut lo = 0.0;
    while n > 0 {
        let x = hi;
        n -= 1;
        let y = partials[n];
        hi = x + y;
        let yr = hi - x;
        lo = y - yr;
        if lo != 0.0 {
            break;
        }
    }
    if n > 0 && ((lo < 0.0 && partials[n - 1] < 0.0) || (lo > 0.0 && partials[n - 1] > 0.0)) {
        let y = lo * 2.0;
        let x = hi + y;
        let yr = x - hi;
        if y == yr {
            hi = x;
        }
    }
    hi
}

/// `round(x, nd)` do Python: arredondamento decimal exato do valor binário, empate ao par.
pub fn py_round(x: f64, nd: i32) -> f64 {
    if !x.is_finite() {
        return x;
    }
    if nd > 323 {
        return x;
    }
    if nd < -308 {
        return 0.0f64.copysign(x);
    }
    if nd >= 0 {
        return format!("{:.*}", nd as usize, x)
            .parse::<f64>()
            .expect("formatted finite float");
    }
    // Work on the exact decimal expansion, avoiding binary division and double rounding.
    let exact = format!("{:.1074}", x.abs());
    let (whole, frac) = exact.split_once('.').expect("fixed decimal");
    let k = (-nd) as usize;
    let padded = format!("{}{}", "0".repeat(k.saturating_sub(whole.len())), whole);
    let cut = padded.len() - k;
    let mut kept: Vec<u8> = padded[..cut].bytes().collect();
    let discarded = &padded.as_bytes()[cut..];
    let greater = discarded[0] > b'5'
        || (discarded[0] == b'5'
            && (discarded[1..].iter().any(|d| *d != b'0') || frac.bytes().any(|d| d != b'0')));
    let tie = discarded[0] == b'5'
        && discarded[1..].iter().all(|d| *d == b'0')
        && frac.bytes().all(|d| d == b'0');
    if greater || (tie && kept.last().is_some_and(|d| (d - b'0') % 2 == 1)) {
        let mut carry = true;
        for digit in kept.iter_mut().rev() {
            if *digit == b'9' {
                *digit = b'0';
            } else {
                *digit += 1;
                carry = false;
                break;
            }
        }
        if carry {
            kept.insert(0, b'1');
        }
    }
    let text = format!(
        "{}{}",
        String::from_utf8(kept).expect("digits"),
        "0".repeat(k)
    );
    text.parse::<f64>().expect("decimal integer").copysign(x)
}

/// `np.round(x, nd)` = `rint(x * 10^nd) / 10^nd`.
pub fn np_round(x: f64, nd: i32) -> f64 {
    let f = 10f64.powi(nd.abs());
    if nd >= 0 {
        (x * f).round_ties_even() / f
    } else {
        (x / f).round_ties_even() * f
    }
}

/// `np.median` (tamanho ímpar: elemento central; par: média dos dois centrais).
pub fn median(xs: &[f64]) -> f64 {
    if xs.iter().any(|x| x.is_nan()) {
        return f64::NAN;
    }
    let mut v = xs.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// `np.add.reduce` de um vetor contíguo de float64: soma em pares do numpy
/// (`pairwise_sum_DOUBLE`, blocos de 8 acumuladores, PW_BLOCKSIZE = 128).
pub fn np_sum(a: &[f64]) -> f64 {
    pairwise(a)
}

fn pairwise(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut res = 0.0_f64;
        for &x in a {
            res += x;
        }
        // numpy usa `res = 0.; res += a[i]`; -0.0 + 0.0 = 0.0 não importa aqui.
        res
    } else if n <= 128 {
        let mut r = [0.0_f64; 8];
        r.copy_from_slice(&a[..8]);
        let mut i = 8;
        while i < n - (n % 8) {
            for j in 0..8 {
                r[j] += a[i + j];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        while i < n {
            res += a[i];
            i += 1;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        pairwise(&a[..n2]) + pairwise(&a[n2..])
    }
}

/// `np.mean` de vetor contíguo (soma em pares / n).
pub fn np_mean(a: &[f64]) -> f64 {
    np_sum(a) / a.len() as f64
}

/// `np.median` (vetor finito): elemento central, ou (a+b)/2 dos dois centrais.
pub fn np_median(a: &[f64]) -> f64 {
    let mut s = a.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

/// `np.quantile(a, q)` método linear (numpy ≥ 1.22): índice virtual (n-1)·q e `_lerp`.
pub fn np_quantile(a: &[f64], q: f64) -> f64 {
    if a.is_empty() || a.iter().any(|x| x.is_nan()) || !(0.0..=1.0).contains(&q) {
        return f64::NAN;
    }
    let mut s = a.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    let virtual_index = (n as f64 - 1.0) * q;
    let prev = virtual_index.floor();
    let gamma = virtual_index - prev;
    let lo = (prev as usize).min(n - 1);
    let hi = (lo + 1).min(n - 1);
    lerp(s[lo], s[hi], gamma)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    let diff = b - a;
    if t >= 0.5 {
        b - diff * (1.0 - t)
    } else {
        a + diff * t
    }
}

pub use np_quantile as quantile_linear;
pub fn nanmedian(a: &[f64]) -> f64 {
    median(
        &a.iter()
            .copied()
            .filter(|x| !x.is_nan())
            .collect::<Vec<_>>(),
    )
}
pub fn nanmean(a: &[f64]) -> f64 {
    let v: Vec<_> = a
        .iter()
        .map(|x| if x.is_nan() { 0.0 } else { *x })
        .collect();
    np_sum(&v) / a.iter().filter(|x| !x.is_nan()).count() as f64
}
