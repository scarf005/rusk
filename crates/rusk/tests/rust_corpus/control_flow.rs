pub fn first_positive(values: &[i32]) -> Option<i32> {
    for value in values {
        if *value > 0 {
            return Some(*value);
        }
    }
    None
}

pub fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if value < min { min } else { if value > max { max } else { value } }
}

pub fn sum_until(values: &[i32], limit: usize) -> i32 {
    let mut index = 0;
    let mut total = 0;
    while index < limit {
        total += values[index];
        index += 1;
    }
    total
}
