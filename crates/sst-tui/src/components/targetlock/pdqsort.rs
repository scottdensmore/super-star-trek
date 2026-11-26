//! Go `sort.Slice` (pattern-defeating quicksort from `sort/zsortfunc.go`),
//! reproduced so that unstable orderings of equal elements match Go.

/// `sort.Slice(data, less)`.
pub fn sort_slice<T>(data: &mut [T], less: impl Fn(&T, &T) -> bool) {
    let n = data.len();
    let limit = bits_len(n);
    let mut d = Data {
        v: data,
        less: &less,
    };
    pdqsort(&mut d, 0, n, limit);
}

struct Data<'a, T, F: Fn(&T, &T) -> bool> {
    v: &'a mut [T],
    less: &'a F,
}

impl<T, F: Fn(&T, &T) -> bool> Data<'_, T, F> {
    fn less(&self, i: usize, j: usize) -> bool {
        (self.less)(&self.v[i], &self.v[j])
    }
    fn swap(&mut self, i: usize, j: usize) {
        self.v.swap(i, j);
    }
}

fn bits_len(n: usize) -> usize {
    (usize::BITS - n.leading_zeros()) as usize
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Hint {
    Unknown,
    Increasing,
    Decreasing,
}

fn insertion_sort<T, F: Fn(&T, &T) -> bool>(d: &mut Data<T, F>, a: usize, b: usize) {
    for i in a + 1..b {
        let mut j = i;
        while j > a && d.less(j, j - 1) {
            d.swap(j, j - 1);
            j -= 1;
        }
    }
}

fn sift_down<T, F: Fn(&T, &T) -> bool>(d: &mut Data<T, F>, lo: usize, hi: usize, first: usize) {
    let mut root = lo;
    loop {
        let mut child = 2 * root + 1;
        if child >= hi {
            break;
        }
        if child + 1 < hi && d.less(first + child, first + child + 1) {
            child += 1;
        }
        if !d.less(first + root, first + child) {
            return;
        }
        d.swap(first + root, first + child);
        root = child;
    }
}

fn heap_sort<T, F: Fn(&T, &T) -> bool>(d: &mut Data<T, F>, a: usize, b: usize) {
    let first = a;
    let lo = 0;
    let hi = b - a;
    let mut i = (hi as isize - 1) / 2;
    while i >= 0 {
        sift_down(d, i as usize, hi, first);
        i -= 1;
    }
    let mut i = hi as isize - 1;
    while i >= 0 {
        d.swap(first, first + i as usize);
        sift_down(d, lo, i as usize, first);
        i -= 1;
    }
}

fn pdqsort<T, F: Fn(&T, &T) -> bool>(
    d: &mut Data<T, F>,
    mut a: usize,
    mut b: usize,
    mut limit: usize,
) {
    const MAX_INSERTION: usize = 12;
    let mut was_balanced = true;
    let mut was_partitioned = true;

    loop {
        let length = b - a;
        if length <= MAX_INSERTION {
            insertion_sort(d, a, b);
            return;
        }
        if limit == 0 {
            heap_sort(d, a, b);
            return;
        }
        if !was_balanced {
            break_patterns(d, a, b);
            limit -= 1;
        }

        let (mut pivot, mut hint) = choose_pivot(d, a, b);
        if hint == Hint::Decreasing {
            reverse_range(d, a, b);
            pivot = (b - 1) - (pivot - a);
            hint = Hint::Increasing;
        }

        if was_balanced
            && was_partitioned
            && hint == Hint::Increasing
            && partial_insertion_sort(d, a, b)
        {
            return;
        }

        if a > 0 && !d.less(a - 1, pivot) {
            let mid = partition_equal(d, a, b, pivot);
            a = mid;
            continue;
        }

        let (mid, already_partitioned) = partition(d, a, b, pivot);
        was_partitioned = already_partitioned;

        let (left_len, right_len) = (mid - a, b - mid);
        let balance_threshold = length / 8;
        if left_len < right_len {
            was_balanced = left_len >= balance_threshold;
            pdqsort(d, a, mid, limit);
            a = mid + 1;
        } else {
            was_balanced = right_len >= balance_threshold;
            pdqsort(d, mid + 1, b, limit);
            b = mid;
        }
    }
}

fn partition<T, F: Fn(&T, &T) -> bool>(
    d: &mut Data<T, F>,
    a: usize,
    b: usize,
    pivot: usize,
) -> (usize, bool) {
    d.swap(a, pivot);
    let (mut i, mut j) = (a as isize + 1, b as isize - 1);
    while i <= j && d.less(i as usize, a) {
        i += 1;
    }
    while i <= j && !d.less(j as usize, a) {
        j -= 1;
    }
    if i > j {
        d.swap(j as usize, a);
        return (j as usize, true);
    }
    d.swap(i as usize, j as usize);
    i += 1;
    j -= 1;
    loop {
        while i <= j && d.less(i as usize, a) {
            i += 1;
        }
        while i <= j && !d.less(j as usize, a) {
            j -= 1;
        }
        if i > j {
            break;
        }
        d.swap(i as usize, j as usize);
        i += 1;
        j -= 1;
    }
    d.swap(j as usize, a);
    (j as usize, false)
}

fn partition_equal<T, F: Fn(&T, &T) -> bool>(
    d: &mut Data<T, F>,
    a: usize,
    b: usize,
    pivot: usize,
) -> usize {
    d.swap(a, pivot);
    let (mut i, mut j) = (a as isize + 1, b as isize - 1);
    loop {
        while i <= j && !d.less(a, i as usize) {
            i += 1;
        }
        while i <= j && d.less(a, j as usize) {
            j -= 1;
        }
        if i > j {
            break;
        }
        d.swap(i as usize, j as usize);
        i += 1;
        j -= 1;
    }
    i as usize
}

fn partial_insertion_sort<T, F: Fn(&T, &T) -> bool>(
    d: &mut Data<T, F>,
    a: usize,
    b: usize,
) -> bool {
    const MAX_STEPS: usize = 5;
    const SHORTEST_SHIFTING: usize = 50;
    let mut i = a + 1;
    for _ in 0..MAX_STEPS {
        while i < b && !d.less(i, i - 1) {
            i += 1;
        }
        if i == b {
            return true;
        }
        if b - a < SHORTEST_SHIFTING {
            return false;
        }
        d.swap(i, i - 1);
        if i - a >= 2 {
            let mut j = i - 1;
            while j >= 1 {
                if !d.less(j, j - 1) {
                    break;
                }
                d.swap(j, j - 1);
                j -= 1;
            }
        }
        if b - i >= 2 {
            let mut j = i + 1;
            while j < b {
                if !d.less(j, j - 1) {
                    break;
                }
                d.swap(j, j - 1);
                j += 1;
            }
        }
    }
    false
}

fn break_patterns<T, F: Fn(&T, &T) -> bool>(d: &mut Data<T, F>, a: usize, b: usize) {
    let length = b - a;
    if length >= 8 {
        let mut random = length as u64;
        let modulus = 1usize << bits_len(length);
        let mut idx = a + (length / 4) * 2 - 1;
        while idx <= a + (length / 4) * 2 + 1 {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            let mut other = (random as usize) & (modulus - 1);
            if other >= length {
                other -= length;
            }
            d.swap(idx, a + other);
            idx += 1;
        }
    }
}

fn choose_pivot<T, F: Fn(&T, &T) -> bool>(d: &mut Data<T, F>, a: usize, b: usize) -> (usize, Hint) {
    const SHORTEST_NINTHER: usize = 50;
    const MAX_SWAPS: usize = 4 * 3;
    let l = b - a;
    let mut swaps = 0;
    let mut i = a + l / 4;
    let mut j = a + l / 4 * 2;
    let mut k = a + l / 4 * 3;
    if l >= 8 {
        if l >= SHORTEST_NINTHER {
            i = median_adjacent(d, i, &mut swaps);
            j = median_adjacent(d, j, &mut swaps);
            k = median_adjacent(d, k, &mut swaps);
        }
        j = median(d, i, j, k, &mut swaps);
    }
    match swaps {
        0 => (j, Hint::Increasing),
        MAX_SWAPS => (j, Hint::Decreasing),
        _ => (j, Hint::Unknown),
    }
}

fn order2<T, F: Fn(&T, &T) -> bool>(
    d: &Data<T, F>,
    a: usize,
    b: usize,
    swaps: &mut usize,
) -> (usize, usize) {
    if d.less(b, a) {
        *swaps += 1;
        return (b, a);
    }
    (a, b)
}

fn median<T, F: Fn(&T, &T) -> bool>(
    d: &Data<T, F>,
    a: usize,
    b: usize,
    c: usize,
    swaps: &mut usize,
) -> usize {
    let (a, b) = order2(d, a, b, swaps);
    let (b, c) = order2(d, b, c, swaps);
    let _ = c;
    let (_a, b) = order2(d, a, b, swaps);
    b
}

fn median_adjacent<T, F: Fn(&T, &T) -> bool>(d: &Data<T, F>, a: usize, swaps: &mut usize) -> usize {
    median(d, a - 1, a, a + 1, swaps)
}

fn reverse_range<T, F: Fn(&T, &T) -> bool>(d: &mut Data<T, F>, a: usize, b: usize) {
    let (mut i, mut j) = (a, b - 1);
    while i < j {
        d.swap(i, j);
        i += 1;
        j -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_like_a_total_order() {
        let mut v: Vec<i32> = (0..200).map(|i| (i * 7919) % 211).collect();
        sort_slice(&mut v, |a, b| a < b);
        assert!(v.windows(2).all(|w| w[0] <= w[1]));
        let mut r: Vec<i32> = (0..100).rev().collect();
        sort_slice(&mut r, |a, b| a < b);
        assert_eq!(r, (0..100).collect::<Vec<_>>());
    }
}
