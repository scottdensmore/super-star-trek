//! Go's `sort.Stable` (insertion sort blocks of 20 + SymMerge), ported
//! exactly so that sorts with non-strict `Less` functions (sahilm/fuzzy uses
//! `>=`) reorder ties exactly like Go.

/// `sort.Stable(data)` where `less(a, b)` is `data.Less(i, j)`.
pub fn stable<T>(data: &mut [T], less: impl Fn(&T, &T) -> bool) {
    let n = data.len();
    let mut block_size = 20;
    let (mut a, mut b) = (0, block_size);
    while b <= n {
        insertion_sort(data, a, b, &less);
        a = b;
        b += block_size;
    }
    insertion_sort(data, a, n, &less);
    while block_size < n {
        a = 0;
        b = 2 * block_size;
        while b <= n {
            sym_merge(data, a, a + block_size, b, &less);
            a = b;
            b += 2 * block_size;
        }
        let m = a + block_size;
        if m < n {
            sym_merge(data, a, m, n, &less);
        }
        block_size *= 2;
    }
}

fn insertion_sort<T>(data: &mut [T], a: usize, b: usize, less: &impl Fn(&T, &T) -> bool) {
    for i in a + 1..b {
        let mut j = i;
        while j > a && less(&data[j], &data[j - 1]) {
            data.swap(j, j - 1);
            j -= 1;
        }
    }
}

fn sym_merge<T>(data: &mut [T], a: usize, m: usize, b: usize, less: &impl Fn(&T, &T) -> bool) {
    if m - a == 1 {
        let mut i = m;
        let mut j = b;
        while i < j {
            let h = (i + j) >> 1;
            if less(&data[h], &data[a]) {
                i = h + 1;
            } else {
                j = h;
            }
        }
        let mut k = a;
        while k + 1 < i {
            data.swap(k, k + 1);
            k += 1;
        }
        return;
    }
    if b - m == 1 {
        let mut i = a;
        let mut j = m;
        while i < j {
            let h = (i + j) >> 1;
            if !less(&data[m], &data[h]) {
                i = h + 1;
            } else {
                j = h;
            }
        }
        let mut k = m;
        while k > i {
            data.swap(k, k - 1);
            k -= 1;
        }
        return;
    }
    let mid = (a + b) >> 1;
    let n = mid + m;
    let (mut start, mut r) = if m > mid { (n - b, mid) } else { (a, m) };
    let p = n - 1;
    while start < r {
        let c = (start + r) >> 1;
        if !less(&data[p - c], &data[c]) {
            start = c + 1;
        } else {
            r = c;
        }
    }
    let end = n - start;
    if start < m && m < end {
        rotate(data, start, m, end);
    }
    if a < start && start < mid {
        sym_merge(data, a, start, mid, less);
    }
    if mid < end && end < b {
        sym_merge(data, mid, end, b, less);
    }
}

fn swap_range<T>(data: &mut [T], a: usize, b: usize, n: usize) {
    for i in 0..n {
        data.swap(a + i, b + i);
    }
}

fn rotate<T>(data: &mut [T], a: usize, m: usize, b: usize) {
    let mut i = m - a;
    let mut j = b - m;
    while i != j {
        if i > j {
            swap_range(data, m - i, m, j);
            i -= j;
        } else {
            swap_range(data, m - i, m + j - i, i);
            j -= i;
        }
    }
    swap_range(data, m - i, m, i);
}

#[cfg(test)]
mod tests {
    #[test]
    fn stable_sorts() {
        let mut v: Vec<(i32, usize)> = (0..100).map(|i| ((i * 37 % 7) as i32, i)).collect();
        super::stable(&mut v, |a, b| a.0 < b.0);
        for w in v.windows(2) {
            assert!(w[0].0 < w[1].0 || (w[0].0 == w[1].0 && w[0].1 < w[1].1));
        }
    }
}
