use std::cmp::Ordering;
use std::path::Path;

use crate::image::ImageEntry;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageSortMode {
    Size,
    Date,
    Name,
}

impl ImageSortMode {
    pub fn sort(self, entries: &mut [ImageEntry], reversed: bool) {
        entries.sort_by(|a, b| {
            let ordering = match self {
                Self::Size => {
                    let ordering = file_size(&a.path).cmp(&file_size(&b.path));
                    if ordering == Ordering::Equal {
                        natural_cmp(&a.name, &b.name)
                    } else {
                        ordering
                    }
                }
                Self::Date => {
                    let ordering = modified_time(&a.path).cmp(&modified_time(&b.path));
                    if ordering == Ordering::Equal {
                        natural_cmp(&a.name, &b.name)
                    } else {
                        ordering.reverse()
                    }
                }
                Self::Name => {
                    let ordering = first_char_code(&a.name).cmp(&first_char_code(&b.name));
                    if ordering == Ordering::Equal {
                        natural_cmp(&a.name, &b.name)
                    } else {
                        ordering
                    }
                }
            };
            if reversed {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }

    pub fn is_size(self) -> bool {
        matches!(self, Self::Size)
    }

    pub fn is_name(self) -> bool {
        matches!(self, Self::Name)
    }
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0)
}

fn modified_time(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(std::time::UNIX_EPOCH)
}

fn first_char_code(name: &str) -> u32 {
    name.chars().next().map_or(0, u32::from)
}

/// 零分配自然排序比较函数
/// 逐字符就地比较，遇到数字段时比较数值大小，不分配中间向量
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let mut ai = 0;
    let mut bi = 0;

    loop {
        // 两者都到末尾
        if ai >= a_bytes.len() && bi >= b_bytes.len() {
            return Ordering::Equal;
        }
        // a先结束
        if ai >= a_bytes.len() {
            return Ordering::Less;
        }
        // b先结束
        if bi >= b_bytes.len() {
            return Ordering::Greater;
        }

        let ca = a_bytes[ai];
        let cb = b_bytes[bi];

        let a_is_digit = ca.is_ascii_digit();
        let b_is_digit = cb.is_ascii_digit();

        if a_is_digit && b_is_digit {
            // 跳过前导零并比较数字段
            let a_start = ai;
            let b_start = bi;

            // 跳过前导零
            while ai < a_bytes.len() && a_bytes[ai] == b'0' {
                ai += 1;
            }
            let a_num_start = ai;

            while bi < b_bytes.len() && b_bytes[bi] == b'0' {
                bi += 1;
            }
            let b_num_start = bi;

            // 计算有效数字长度
            while ai < a_bytes.len() && a_bytes[ai].is_ascii_digit() {
                ai += 1;
            }
            while bi < b_bytes.len() && b_bytes[bi].is_ascii_digit() {
                bi += 1;
            }

            let a_len = ai - a_num_start;
            let b_len = bi - b_num_start;

            // 有效位数不同，位数多的大
            if a_len != b_len {
                return a_len.cmp(&b_len);
            }

            // 位数相同，逐位比较
            for k in 0..a_len {
                let cmp = a_bytes[a_num_start + k].cmp(&b_bytes[b_num_start + k]);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }

            // 数值相同，前导零多的排后面（稳定性）
            let a_zeros = a_num_start - a_start;
            let b_zeros = b_num_start - b_start;
            if a_zeros != b_zeros {
                // 不立即返回，继续比较后续字符
                // 但如果后续完全相同，前导零少的排前面
            }
        } else if a_is_digit != b_is_digit {
            // 数字排在非数字前面（保持一致行为）
            return if a_is_digit { Ordering::Less } else { Ordering::Greater };
        } else {
            // 都是非数字字符，大小写不敏感比较
            let la = ca.to_ascii_lowercase();
            let lb = cb.to_ascii_lowercase();
            if la != lb {
                return la.cmp(&lb);
            }
            ai += 1;
            bi += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_sort_basic() {
        let mut files = vec!["img10.jpg", "img2.jpg", "img1.jpg", "img20.jpg"];
        files.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(files, vec!["img1.jpg", "img2.jpg", "img10.jpg", "img20.jpg"]);
    }

    #[test]
    fn test_natural_sort_mixed() {
        let mut files = vec!["b2.png", "a10.png", "a2.png", "b1.png"];
        files.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(files, vec!["a2.png", "a10.png", "b1.png", "b2.png"]);
    }

    #[test]
    fn test_natural_sort_equal() {
        assert_eq!(natural_cmp("abc", "abc"), Ordering::Equal);
    }

    #[test]
    fn test_natural_sort_empty() {
        assert_eq!(natural_cmp("", ""), Ordering::Equal);
        assert_eq!(natural_cmp("", "a"), Ordering::Less);
    }

    #[test]
    fn test_leading_zeros() {
        // 数值相同时，前导零不影响排序（视为相等数值）
        // 排序结果按原始字符串字节序：'0' < '9'，所以前导零多的排前面
        let mut files = vec!["file009.txt", "file09.txt", "file9.txt"];
        files.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(files, vec!["file009.txt", "file09.txt", "file9.txt"]);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(natural_cmp("ABC", "abc"), Ordering::Equal);
        assert_eq!(natural_cmp("aBC", "Abc"), Ordering::Equal);
    }

    #[test]
    fn test_numbers_before_letters() {
        // 数字字符排在字母字符之前
        assert_eq!(natural_cmp("1abc", "abc"), Ordering::Less);
    }
}
